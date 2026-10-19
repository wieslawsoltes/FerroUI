//! `KeyedDisposable<TKey>` (`Internal/KeyedDisposable.cs`): disposables tracked by key, each
//! disposed when it is replaced, removed, or when the tracker is disposed.
#![allow(dead_code)]

use ferroui_system::collections::generic::Dictionary;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

/// The tracker (`KeyedDisposable<TKey>`).
pub(crate) struct KeyedDisposable<K> {
    disposables: RefCell<Dictionary<K, Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl<K: Eq + Hash + Clone> Default for KeyedDisposable<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Clone> KeyedDisposable<K> {
    pub(crate) fn new() -> Self {
        Self { disposables: RefCell::new(Dictionary::new()), disposed: Cell::new(false) }
    }

    /// The number of disposables tracked (`Count`).
    pub(crate) fn count(&self) -> usize {
        self.disposables.borrow().len()
    }

    /// The keys tracked (`Keys`).
    pub(crate) fn keys(&self) -> Vec<K> {
        self.disposables.borrow().keys().cloned().collect()
    }

    /// Whether a disposable is tracked for `key` (`ContainsKey`).
    pub(crate) fn contains_key(&self, key: &K) -> bool {
        self.disposables.borrow().contains_key(key)
    }

    /// Whether the tracker has been disposed (`IsDisposed`).
    pub(crate) fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// Tracks `item` for `key` when it is a disposable, disposing the one it replaces; an item that
    /// is not a disposable (`None`) removes and disposes what `key` tracked
    /// (`Add<TItem>(TKey, TItem)`). Once the tracker is disposed, a disposable is disposed at once.
    pub(crate) fn add(&self, key: K, item: Option<Rc<dyn IDisposable>>) {
        let Some(disposable) = item else {
            self.remove(&key);
            return;
        };
        let to_dispose = if self.disposed.get() {
            Some(disposable)
        } else {
            let mut disposables = self.disposables.borrow_mut();
            let same = disposables.get(&key).map(|existing| same_disposable(existing, &disposable));
            match same {
                Some(true) => return,
                Some(false) => disposables.insert(key, disposable),
                None => {
                    disposables.insert(key, disposable);
                    None
                }
            }
        };
        if let Some(to_dispose) = to_dispose {
            to_dispose.dispose();
        }
    }

    /// Removes and disposes what `key` tracked (`Remove`).
    pub(crate) fn remove(&self, key: &K) {
        let removed = self.disposables.borrow_mut().remove(key);
        if let Some(removed) = removed {
            removed.dispose();
        }
    }

    /// Disposes every disposable tracked, once (`Dispose`). Every one is disposed even when some
    /// panic; the panics are then reported as one (`AggregateException`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): the `AggregateException` of the inner exceptions is a
    /// panic whose message counts them.
    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let snapshot: Vec<Rc<dyn IDisposable>> = {
            let mut disposables = self.disposables.borrow_mut();
            let snapshot = disposables.values().cloned().collect();
            disposables.clear();
            snapshot
        };
        let mut errors = 0usize;
        for disposable in snapshot {
            if panic::catch_unwind(AssertUnwindSafe(|| disposable.dispose())).is_err() {
                errors += 1;
            }
        }
        if errors > 0 {
            panic!("AggregateException: {errors} of the disposables panicked when disposed");
        }
    }
}

impl<K: Eq + Hash + Clone> IDisposable for KeyedDisposable<K> {
    fn dispose(&self) {
        KeyedDisposable::dispose(self);
    }
}

/// `ReferenceEquals` of two disposables.
fn same_disposable(left: &Rc<dyn IDisposable>, right: &Rc<dyn IDisposable>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(left), Rc::as_ptr(right))
}
