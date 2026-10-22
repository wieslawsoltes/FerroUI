use super::folding_events::{FoldingEvents, FoldingSubscription};
use super::{FoldingSection, NewFolding};
use crate::document::{DocumentError, TextDocument};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub enum FoldingError {
    Document(DocumentError),
    InvalidRange,
    Unsorted,
}
impl fmt::Display for FoldingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(e) => e.fmt(f),
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

pub struct FoldingManager {
    document: TextDocument,
    foldings: RefCell<Vec<FoldingSection>>,
    first_update: Cell<bool>,
    events: Rc<FoldingEvents>,
}
impl FoldingManager {
    pub fn new(document: TextDocument) -> Self {
        Self {
            document,
            foldings: RefCell::new(Vec::new()),
            first_update: Cell::new(true),
            events: Rc::default(),
        }
    }
    pub fn document(&self) -> &TextDocument {
        &self.document
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> FoldingSubscription {
        self.events.subscribe(handler)
    }
    pub fn create_folding(&self, start: usize, end: usize) -> Result<FoldingSection, FoldingError> {
        if start >= end || end > self.document.text_length() {
            return Err(FoldingError::InvalidRange);
        }
        let section = FoldingSection::new(&self.document, start, end, &self.events)?;
        self.foldings.borrow_mut().push(section.clone());
        self.foldings
            .borrow_mut()
            .sort_by_key(|s| (s.start_offset(), s.end_offset()));
        self.events.notify();
        Ok(section)
    }
    pub fn remove_folding(&self, section: &FoldingSection) {
        let _batch = self.events.batch();
        section.set_is_folded(false);
        self.foldings.borrow_mut().retain(|s| !s.ptr_eq(section));
        self.events.notify();
    }
    pub fn clear(&self) {
        let _batch = self.events.batch();
        for section in self.all_foldings() {
            section.set_is_folded(false);
        }
        self.foldings.borrow_mut().clear();
        self.events.notify();
    }
    pub fn all_foldings(&self) -> Vec<FoldingSection> {
        self.foldings.borrow_mut().retain(|s| s.length() > 0);
        self.foldings.borrow().clone()
    }
    pub fn get_next_folding(&self, offset: usize) -> Option<FoldingSection> {
        self.all_foldings()
            .into_iter()
            .find(|s| s.start_offset() >= offset)
    }
    pub fn get_next_folded_folding_start(&self, offset: usize) -> Option<usize> {
        self.all_foldings()
            .into_iter()
            .find(|s| s.is_folded() && s.start_offset() >= offset)
            .map(|s| s.start_offset())
    }
    pub fn get_foldings_at(&self, offset: usize) -> Vec<FoldingSection> {
        self.all_foldings()
            .into_iter()
            .filter(|s| s.start_offset() == offset)
            .collect()
    }
    pub fn get_foldings_containing(&self, offset: usize) -> Vec<FoldingSection> {
        self.all_foldings()
            .into_iter()
            .filter(|s| s.start_offset() <= offset && offset <= s.end_offset())
            .collect()
    }
    pub fn unfold_at(&self, offset: usize) {
        let _batch = self.events.batch();
        for section in self.get_foldings_containing(offset) {
            if section.start_offset() < offset && offset < section.end_offset() {
                section.set_is_folded(false);
            }
        }
    }
    pub fn set_all_folded(&self, folded: bool, definitions_only: bool) {
        let _batch = self.events.batch();
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
        let _batch = self.events.batch();
        let old = self.all_foldings();
        let mut old_index = 0;
        let mut next = Vec::new();
        for region in new.iter().filter(|s| s.start_offset < s.end_offset) {
            while old_index < old.len() && old[old_index].start_offset() < region.start_offset {
                old[old_index].set_is_folded(false);
                old_index += 1;
            }
            let section =
                if old_index < old.len() && old[old_index].start_offset() == region.start_offset {
                    let section = old[old_index].clone();
                    old_index += 1;
                    section.set_end_offset(&self.document, region.end_offset)?;
                    section
                } else {
                    let section = FoldingSection::new(
                        &self.document,
                        region.start_offset,
                        region.end_offset,
                        &self.events,
                    )?;
                    if self.first_update.get() {
                        section.set_is_folded(region.default_closed);
                    }
                    section
                };
            section.set_title(region.name.clone());
            section.set_is_definition(region.is_definition);
            next.push(section);
        }
        for section in old.into_iter().skip(old_index) {
            if first_error_offset.is_some_and(|offset| section.start_offset() >= offset) {
                next.push(section);
            } else {
                section.set_is_folded(false);
            }
        }
        *self.foldings.borrow_mut() = next;
        self.first_update.set(false);
        self.events.notify();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(
            manager
                .update_foldings(&[NewFolding::new(2, 5), NewFolding::new(1, 6)], None)
                .is_err()
        );
        assert_eq!(manager.all_foldings().len(), 1);
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
