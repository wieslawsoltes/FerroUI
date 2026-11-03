//! Local shared storage for computed keyboard rows and keys.
//! Revisions reject reentrant edits at traversal boundaries; no collection borrow
//! spans key computation or caller dispatch.
use super::ResourceTextListError;
use std::{cell::RefCell, rc::Rc};
type Result<T> = std::result::Result<T, ResourceTextListError>;

struct State<T> {
    values: Vec<Option<T>>,
    revision: u64,
}
#[derive(Clone)]
pub struct ComputedList<T: Clone>(Rc<RefCell<State<T>>>);
impl<T: Clone> Default for ComputedList<T> {
    fn default() -> Self {
        Self::from_nullable(Vec::new())
    }
}
impl<T: Clone> ComputedList<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self::from_nullable(values.into_iter().map(Some).collect())
    }
    pub fn from_nullable(values: Vec<Option<T>>) -> Self {
        Self(Rc::new(RefCell::new(State {
            values,
            revision: 0,
        })))
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn len(&self) -> usize {
        self.0.borrow().values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.borrow().values.is_empty()
    }
    pub fn get_nullable(&self, index: usize) -> Result<Option<T>> {
        self.0
            .borrow()
            .values
            .get(index)
            .cloned()
            .ok_or(ResourceTextListError::IndexOutOfRange)
    }
    pub fn get(&self, index: usize) -> Result<T> {
        self.get_nullable(index)?
            .ok_or(ResourceTextListError::NullReference)
    }
    pub fn set_nullable(&self, index: usize, value: Option<T>) -> Result<()> {
        let mut state = self.0.borrow_mut();
        let slot = state
            .values
            .get_mut(index)
            .ok_or(ResourceTextListError::IndexOutOfRange)?;
        let previous = std::mem::replace(slot, value);
        state.revision = state.revision.wrapping_add(1);
        drop(state);
        drop(previous);
        Ok(())
    }
    pub fn set(&self, index: usize, value: T) -> Result<()> {
        self.set_nullable(index, Some(value))
    }
    pub fn push_nullable(&self, value: Option<T>) {
        let mut state = self.0.borrow_mut();
        state.values.push(value);
        state.revision = state.revision.wrapping_add(1);
    }
    pub fn push(&self, value: T) {
        self.push_nullable(Some(value));
    }
    pub fn insert_nullable(&self, index: usize, value: Option<T>) -> Result<()> {
        let mut state = self.0.borrow_mut();
        if index > state.values.len() {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        state.values.insert(index, value);
        state.revision = state.revision.wrapping_add(1);
        Ok(())
    }
    pub fn insert(&self, index: usize, value: T) -> Result<()> {
        self.insert_nullable(index, Some(value))
    }
    pub fn remove(&self, index: usize) -> Result<Option<T>> {
        let mut state = self.0.borrow_mut();
        if index >= state.values.len() {
            return Err(ResourceTextListError::IndexOutOfRange);
        }
        let value = state.values.remove(index);
        state.revision = state.revision.wrapping_add(1);
        Ok(value)
    }
    pub fn clear(&self) {
        let mut state = self.0.borrow_mut();
        let previous = std::mem::take(&mut state.values);
        state.revision = state.revision.wrapping_add(1);
        drop(state);
        drop(previous);
    }
    pub fn snapshot_nullable(&self) -> Vec<Option<T>> {
        self.0.borrow().values.clone()
    }
    pub fn snapshot(&self) -> Result<Vec<T>> {
        self.snapshot_nullable()
            .into_iter()
            .map(|value| value.ok_or(ResourceTextListError::NullReference))
            .collect()
    }
    pub fn iter_nullable(&self) -> ComputedItems<T> {
        let state = self.0.borrow();
        ComputedItems {
            owner: self.clone(),
            revision: state.revision,
            index: 0,
            exhausted: state.values.is_empty(),
        }
    }
    pub fn iter(&self) -> ComputedListIterator<T> {
        ComputedListIterator(self.iter_nullable())
    }
}

/// A Rust iterator over shared computed values. Each item is owned, so callbacks
/// may edit the layout; the next affected traversal step then reports the change.
pub struct ComputedItems<T: Clone> {
    owner: ComputedList<T>,
    revision: u64,
    index: usize,
    exhausted: bool,
}
impl<T: Clone> Iterator for ComputedItems<T> {
    type Item = Result<Option<T>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        let state = self.owner.0.borrow();
        if self.revision != state.revision {
            return Some(Err(ResourceTextListError::Modified));
        }
        match state.values.get(self.index) {
            Some(value) => {
                self.index += 1;
                Some(Ok(value.clone()))
            }
            None => {
                self.exhausted = true;
                None
            }
        }
    }
}
impl<T: Clone> std::iter::FusedIterator for ComputedItems<T> {}
pub struct ComputedListIterator<T: Clone>(ComputedItems<T>);
impl<T: Clone> Iterator for ComputedListIterator<T> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|value| value.and_then(|value| value.ok_or(ResourceTextListError::NullReference)))
    }
}
impl<T: Clone> std::iter::FusedIterator for ComputedListIterator<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Weak};
    #[derive(Clone)]
    struct ObserveDrop {
        owner: Weak<RefCell<State<Self>>>,
        observed: Rc<Cell<Option<usize>>>,
    }
    impl Drop for ObserveDrop {
        fn drop(&mut self) {
            if let Some(owner) = self.owner.upgrade() {
                self.observed.set(Some(
                    owner
                        .try_borrow()
                        .expect("drop after releasing row borrow")
                        .values
                        .len(),
                ));
            }
        }
    }
    #[test]
    fn removed_and_replaced_computed_values_drop_after_releasing_storage_borrow() {
        let values = ComputedList::default();
        let observed = Rc::new(Cell::new(None));
        values.push(ObserveDrop {
            owner: Rc::downgrade(&values.0),
            observed: observed.clone(),
        });
        values.set_nullable(0, None).unwrap();
        assert_eq!(observed.get(), Some(1));
        observed.set(None);
        values.push(ObserveDrop {
            owner: Rc::downgrade(&values.0),
            observed: observed.clone(),
        });
        let removed = values.remove(1).unwrap();
        assert_eq!(observed.get(), None);
        drop(removed);
        assert_eq!(observed.get(), Some(1));
        observed.set(None);
        values.push(ObserveDrop {
            owner: Rc::downgrade(&values.0),
            observed: observed.clone(),
        });
        values.clear();
        assert_eq!(observed.get(), Some(0));
    }
}
