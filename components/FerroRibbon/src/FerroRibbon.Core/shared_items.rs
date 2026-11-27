//! Non-notifying, shared items used by state, queries and native presentations.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Clone the `Rc` to share mutations; use `Rc::ptr_eq` and `Rc::downgrade` for
/// identity and weak ownership. Contents are ordinary Rust values in a `Vec`.
pub struct RibbonItems<T> {
    values: RefCell<Vec<T>>,
    revision: Cell<u64>,
}
impl<T> Default for RibbonItems<T> {
    fn default() -> Self {
        Self {
            values: RefCell::new(Vec::new()),
            revision: Cell::new(0),
        }
    }
}
impl<T> PartialEq for RibbonItems<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<T> Eq for RibbonItems<T> {}
impl<T> RibbonItems<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    pub fn from_values(values: Vec<T>) -> Rc<Self> {
        Rc::new(Self {
            values: RefCell::new(values),
            revision: Cell::new(0),
        })
    }
    fn changed(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
    }
    pub fn len(&self) -> usize {
        self.values.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.borrow().is_empty()
    }
    pub fn set(&self, index: usize, value: T) {
        let old = std::mem::replace(&mut self.values.borrow_mut()[index], value);
        self.changed();
        drop(old);
    }
    pub fn add(&self, value: T) {
        self.values.borrow_mut().push(value);
        self.changed();
    }
    pub fn insert(&self, index: usize, value: T) {
        self.values.borrow_mut().insert(index, value);
        self.changed();
    }
    pub fn remove_at(&self, index: usize) {
        let removed = self.values.borrow_mut().remove(index);
        self.changed();
        drop(removed);
    }
    pub fn clear(&self) {
        let mut old = std::mem::take(&mut *self.values.borrow_mut());
        self.changed();
        old.clear();
        // Reuse the buffer on the ordinary path; retain callback-added values.
        let empty = self.values.borrow().is_empty();
        if empty {
            let unused = std::mem::replace(&mut *self.values.borrow_mut(), old);
            drop(unused);
        }
    }
    // Only fixed, pure predicates over sealed state classes call this helper.
    pub(crate) fn remove_all_state(&self, predicate: impl Fn(&T) -> bool) {
        let removed: Vec<_> = self
            .values
            .borrow_mut()
            .extract_if(.., |v| predicate(v))
            .collect();
        if !removed.is_empty() {
            self.changed();
        }
        drop(removed);
    }
}
impl<T: Clone> RibbonItems<T> {
    pub fn get(&self, index: usize) -> T {
        self.values.borrow()[index].clone()
    }
    pub fn to_vec(&self) -> Vec<T> {
        self.values.borrow().clone()
    }
    /// Owning live iteration. No storage borrow crosses a caller callback.
    /// Mutation invalidates an active nonempty query, including after its end.
    /// A query started empty stays empty. These are product query contracts.
    pub fn iter(self: &Rc<Self>) -> impl Iterator<Item = T> + use<T> {
        let owner = (!self.is_empty()).then(|| self.clone());
        let revision = self.revision.get();
        let mut index = 0;
        std::iter::from_fn(move || {
            let owner = owner.as_ref()?;
            assert_eq!(
                revision,
                owner.revision.get(),
                "ribbon items changed during iteration"
            );
            let value = owner.values.borrow().get(index).cloned();
            if value.is_some() {
                index += 1;
            }
            value
        })
    }
}
impl<T: PartialEq> RibbonItems<T> {
    pub fn remove(&self, value: &T) -> bool {
        let index = self.values.borrow().iter().position(|item| item == value);
        if let Some(index) = index {
            self.remove_at(index);
            true
        } else {
            false
        }
    }
}
