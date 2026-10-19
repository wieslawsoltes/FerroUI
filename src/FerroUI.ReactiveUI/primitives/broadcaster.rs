//! `Broadcaster<T>` (ReactiveUI.Primitives, `Signals/Broadcaster{T}.cs`): the list of observers
//! that the streams of a reactive command notify.
//!
//! Upstream keeps the observers in a copy-on-write array that `Next`, `Error` and `Completed` read
//! as a snapshot, so an observer added or removed during a notification takes effect from the next
//! one. This port keeps that: the list is a shared `Vec` replaced on each change, and a
//! notification walks the snapshot it read, with no borrow held while an observer runs.

use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The observers of a stream, notified from a snapshot of the list (`Broadcaster<T>`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream removes an observer by reference
/// (`Remove(IObserver<T>)`); here [`add`](Self::add) returns an id and [`remove`](Self::remove)
/// takes it, so disposing the same subscription twice removes it once.
pub(crate) struct Broadcaster<T> {
    observers: RefCell<Rc<Vec<(u64, Rc<dyn IObserver<T>>)>>>,
    next_id: Cell<u64>,
}

impl<T> Default for Broadcaster<T> {
    fn default() -> Self {
        Self { observers: RefCell::new(Rc::new(Vec::new())), next_id: Cell::new(0) }
    }
}

impl<T> Broadcaster<T> {
    /// Whether any observer is subscribed (`HasObservers`).
    pub(crate) fn has_observers(&self) -> bool {
        !self.observers.borrow().is_empty()
    }

    /// Adds `observer`, and returns the id that removes it (`Add`).
    pub(crate) fn add(&self, observer: Rc<dyn IObserver<T>>) -> u64 {
        let id = self.next_id.get() + 1;
        self.next_id.set(id);
        let mut observers = self.observers.borrow_mut();
        let mut next: Vec<_> = observers.iter().cloned().collect();
        next.push((id, observer));
        *observers = Rc::new(next);
        id
    }

    /// Removes the observer added with `id`, if it is still there (`Remove`).
    pub(crate) fn remove(&self, id: u64) {
        let mut observers = self.observers.borrow_mut();
        if let Some(index) = observers.iter().position(|(entry, _)| *entry == id) {
            let mut next: Vec<_> = observers.iter().cloned().collect();
            next.remove(index);
            *observers = Rc::new(next);
        }
    }

    /// Removes every observer (`Clear`).
    #[allow(dead_code)]
    pub(crate) fn clear(&self) {
        *self.observers.borrow_mut() = Rc::new(Vec::new());
    }

    fn snapshot(&self) -> Rc<Vec<(u64, Rc<dyn IObserver<T>>)>> {
        self.observers.borrow().clone()
    }

    /// Notifies `value` to the observers of the snapshot (`Next`).
    pub(crate) fn next(&self, value: T)
    where
        T: Clone,
    {
        let snapshot = self.snapshot();
        if let [(_, single)] = snapshot.as_slice() {
            single.on_next(value);
            return;
        }
        for (_, observer) in snapshot.iter() {
            observer.on_next(value.clone());
        }
    }

    /// Notifies `error` to the observers of the snapshot (`Error`).
    #[allow(dead_code)]
    pub(crate) fn error(&self, error: ObservableError) {
        for (_, observer) in self.snapshot().iter() {
            observer.on_error(error.clone());
        }
    }

    /// Notifies the end to the observers of the snapshot (`Completed`).
    #[allow(dead_code)]
    pub(crate) fn completed(&self) {
        for (_, observer) in self.snapshot().iter() {
            observer.on_completed();
        }
    }
}
