use super::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A group of disposables that are disposed together.
#[derive(Default)]
pub struct CompositeDisposable {
    disposables: RefCell<Vec<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl CompositeDisposable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self { disposables: RefCell::new(Vec::with_capacity(capacity)), disposed: Cell::new(false) }
    }

    pub fn from_disposables(disposables: impl IntoIterator<Item = Rc<dyn IDisposable>>) -> Self {
        Self { disposables: RefCell::new(disposables.into_iter().collect()), disposed: Cell::new(false) }
    }

    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    pub fn count(&self) -> usize {
        self.disposables.borrow().len()
    }

    /// Adds a disposable to the group, or disposes it immediately if the group
    /// has already been disposed.
    pub fn add(&self, item: Rc<dyn IDisposable>) {
        if self.disposed.get() {
            item.dispose();
        } else {
            self.disposables.borrow_mut().push(item);
        }
    }

    /// Removes and disposes the first occurrence of `item`.
    pub fn remove(&self, item: &Rc<dyn IDisposable>) -> bool {
        let removed = {
            let mut list = self.disposables.borrow_mut();
            list.iter().position(|d| Rc::ptr_eq(d, item)).map(|i| list.remove(i))
        };
        match removed {
            Some(d) => {
                d.dispose();
                true
            }
            None => false,
        }
    }

    /// Whether the group contains `item`. A disposed group is empty.
    pub fn contains(&self, item: &Rc<dyn IDisposable>) -> bool {
        if self.disposed.get() {
            return false;
        }

        self.disposables.borrow().iter().any(|d| Rc::ptr_eq(d, item))
    }

    /// Copies the disposables of the group over the items of `array` that
    /// start at `array_index`. A disposed group copies nothing.
    ///
    /// Panics when `array_index` is outside of the array or the array has
    /// not enough room after it.
    pub fn copy_to(&self, array: &mut [Rc<dyn IDisposable>], array_index: usize) {
        if array_index >= array.len() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'arrayIndex')");
        }

        // disposed composites are always empty
        if self.disposed.get() {
            return;
        }

        let disposables = self.disposables.borrow();
        if array_index + disposables.len() > array.len() {
            // there is not enough space beyond array_index
            // to accommodate all disposables in this composite
            panic!("Specified argument was out of the range of valid values. (Parameter 'arrayIndex')");
        }

        for (i, d) in disposables.iter().enumerate() {
            array[array_index + i] = d.clone();
        }
    }

    /// Always false.
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Enumerates a copy of the disposables of the group. A disposed group
    /// is empty.
    pub fn get_enumerator(&self) -> std::vec::IntoIter<Rc<dyn IDisposable>> {
        if self.disposed.get() {
            return Vec::new().into_iter();
        }

        // the copy is unavoidable
        self.disposables.borrow().clone().into_iter()
    }

    /// Disposes and removes all items without disposing the group itself.
    pub fn clear(&self) {
        let items = std::mem::take(&mut *self.disposables.borrow_mut());
        for d in items {
            d.dispose();
        }
    }
}

impl IDisposable for CompositeDisposable {
    fn dispose(&self) {
        if !self.disposed.replace(true) {
            self.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;
    use crate::reactive::Disposable;

    #[test]
    fn contains_enumerates_and_copies_its_disposables() {
        let first = Disposable::create(|| {});
        let second = Disposable::create(|| {});
        let other = Disposable::create(|| {});
        let target = CompositeDisposable::from_disposables([first.clone(), second.clone()]);

        assert!(!target.is_read_only());
        assert!(target.contains(&first));
        assert!(!target.contains(&other));

        let items: Vec<_> = target.get_enumerator().collect();
        assert_eq!(2, items.len());
        assert!(Rc::ptr_eq(&items[0], &first) && Rc::ptr_eq(&items[1], &second));

        let mut array = vec![other.clone(), other.clone(), other.clone()];
        target.copy_to(&mut array, 1);
        assert!(Rc::ptr_eq(&array[0], &other));
        assert!(Rc::ptr_eq(&array[1], &first));
        assert!(Rc::ptr_eq(&array[2], &second));

        target.dispose();
        assert!(!target.contains(&first));
        assert_eq!(0, target.get_enumerator().count());
    }

    #[test]
    #[should_panic(expected = "arrayIndex")]
    fn copy_to_throws_without_room() {
        let item = Disposable::create(|| {});
        let target = CompositeDisposable::from_disposables([item.clone(), item.clone()]);
        let mut array = vec![item.clone(), item.clone()];
        target.copy_to(&mut array, 1);
    }
}
