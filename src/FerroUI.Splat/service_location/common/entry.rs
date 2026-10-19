//! `ArrayHelpers`: the list of registrations of one key (`ArrayHelpers.Entry<TValue>`) and the
//! materialization of registrations.

use super::Registration;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The values of one key, in the order they were added, with a snapshot that readers iterate
/// without holding a borrow of the list (`ArrayHelpers.Entry<TValue>`): a factory or a callback
/// that runs over a snapshot may add to or remove from the entry.
///
/// The snapshot is cached until the next change, as upstream caches it by version.
// Deviation (DEVIATIONS.md, Splat): single-threaded; the lock and the volatile reads of upstream
// are cells.
pub(crate) struct Entry<T> {
    list: RefCell<Vec<T>>,
    snapshot: RefCell<Option<Rc<[T]>>>,
    version: Cell<u64>,
    snapshot_version: Cell<u64>,
}

impl<T> Default for Entry<T> {
    fn default() -> Self {
        Self {
            list: RefCell::new(Vec::with_capacity(4)),
            snapshot: RefCell::new(None),
            version: Cell::new(0),
            snapshot_version: Cell::new(0),
        }
    }
}

#[allow(dead_code)]
impl<T: Clone> Entry<T> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// `Count`.
    pub(crate) fn count(&self) -> usize {
        self.list.borrow().len()
    }

    /// `HasItems`.
    pub(crate) fn has_items(&self) -> bool {
        !self.list.borrow().is_empty()
    }

    /// `Add`.
    pub(crate) fn add(&self, value: T) {
        self.list.borrow_mut().push(value);
        self.bump_version();
    }

    /// `RemoveCurrent`: removes the last value; `true` when that left the entry empty.
    pub(crate) fn remove_current(&self) -> bool {
        let remaining = {
            let mut list = self.list.borrow_mut();
            if list.pop().is_none() {
                return false;
            }
            list.len()
        };
        self.bump_version();
        if remaining != 0 {
            return false;
        }
        self.publish_empty_snapshot();
        true
    }

    /// `Remove`: removes the first value for which `matches` holds; `true` when one was removed.
    pub(crate) fn remove_where(&self, matches: impl Fn(&T) -> bool) -> bool {
        let remaining = {
            let mut list = self.list.borrow_mut();
            let Some(index) = list.iter().position(matches) else {
                return false;
            };
            list.remove(index);
            list.len()
        };
        self.bump_version();
        if remaining == 0 {
            self.publish_empty_snapshot();
        }
        true
    }

    /// `Clear`.
    pub(crate) fn clear(&self) {
        self.list.borrow_mut().clear();
        self.bump_version();
        self.publish_empty_snapshot();
    }

    /// `GetSnapshot`: the values, the same snapshot until the entry changes.
    pub(crate) fn get_snapshot(&self) -> Rc<[T]> {
        if self.snapshot_version.get() == self.version.get() {
            if let Some(snapshot) = self.snapshot.borrow().as_ref() {
                return snapshot.clone();
            }
        }
        let snapshot: Rc<[T]> = self.list.borrow().iter().cloned().collect();
        *self.snapshot.borrow_mut() = Some(snapshot.clone());
        self.snapshot_version.set(self.version.get());
        snapshot
    }

    /// `CopyItemsTo`.
    pub(crate) fn copy_items_to(&self, destination: &mut Vec<T>) {
        destination.extend(self.list.borrow().iter().cloned());
    }

    fn bump_version(&self) {
        self.version.set(self.version.get() + 1);
    }

    fn publish_empty_snapshot(&self) {
        let empty: Rc<[T]> = Rc::from(Vec::new());
        *self.snapshot.borrow_mut() = Some(empty);
        self.snapshot_version.set(self.version.get());
    }
}

/// `MaterializeRegistrations`: the value of every registration, in order, each factory invoked
/// once; the registrations that give `None` are left out.
pub(crate) fn materialize_registrations<T: Clone>(registrations: &[Registration<T>]) -> Vec<T> {
    let mut result = Vec::with_capacity(registrations.len());
    for registration in registrations {
        if let Some(value) = registration.resolve() {
            result.push(value);
        }
    }
    result
}
