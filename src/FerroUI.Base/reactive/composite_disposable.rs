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
