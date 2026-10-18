//! `CompositeDisposable`.

use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A group of disposables that are disposed together (`CompositeDisposable`).
///
/// Disposables compare by identity (`Remove`, `Contains`). A disposable added after the group was
/// disposed is disposed at once. The group disposes its members outside of any borrow of its list,
/// so that a member may add to or remove from the group while it is disposed.
#[derive(Default)]
pub struct CompositeDisposable {
    disposables: RefCell<Vec<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl CompositeDisposable {
    /// An empty group.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// An empty group with room for `capacity` disposables.
    pub fn with_capacity(capacity: usize) -> Rc<Self> {
        Rc::new(Self { disposables: RefCell::new(Vec::with_capacity(capacity)), disposed: Cell::new(false) })
    }

    /// A group of the given disposables.
    pub fn from_disposables(disposables: impl IntoIterator<Item = Rc<dyn IDisposable>>) -> Rc<Self> {
        Rc::new(Self { disposables: RefCell::new(disposables.into_iter().collect()), disposed: Cell::new(false) })
    }

    /// The number of disposables in the group.
    pub fn count(&self) -> usize {
        self.disposables.borrow().len()
    }

    /// Adds a disposable; disposes it at once when the group has been disposed.
    pub fn add(&self, item: Rc<dyn IDisposable>) {
        if self.disposed.get() {
            item.dispose();
            return;
        }
        self.disposables.borrow_mut().push(item);
    }

    /// Removes the disposable and disposes it; false when the group does not hold it.
    pub fn remove(&self, item: &Rc<dyn IDisposable>) -> bool {
        if self.disposed.get() {
            return false;
        }
        let removed = {
            let mut disposables = self.disposables.borrow_mut();
            disposables.iter().position(|d| same(d, item)).map(|index| disposables.remove(index))
        };
        match removed {
            Some(removed) => {
                removed.dispose();
                true
            }
            None => false,
        }
    }

    /// Disposes the disposables and removes them, without disposing the group (`Clear`).
    pub fn clear(&self) {
        if self.disposed.get() {
            return;
        }
        let disposables = std::mem::take(&mut *self.disposables.borrow_mut());
        for d in disposables {
            d.dispose();
        }
    }

    /// Whether the group holds the disposable.
    pub fn contains(&self, item: &Rc<dyn IDisposable>) -> bool {
        !self.disposed.get() && self.disposables.borrow().iter().any(|d| same(d, item))
    }

    /// The disposables of the group, in the order they were added (`ToArray`, the enumerator).
    pub fn to_vec(&self) -> Vec<Rc<dyn IDisposable>> {
        if self.disposed.get() {
            return Vec::new();
        }
        self.disposables.borrow().clone()
    }
}

fn same(a: &Rc<dyn IDisposable>, b: &Rc<dyn IDisposable>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

impl IDisposable for CompositeDisposable {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let disposables = std::mem::take(&mut *self.disposables.borrow_mut());
        for d in disposables {
            d.dispose();
        }
    }
}

impl ICancelable for CompositeDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
