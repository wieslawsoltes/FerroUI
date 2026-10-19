//! `AppliedChangeObservable`.

use crate::binding::bindings::BindingChange;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

/// The changes a two-way binding actually wrote, handed to whoever subscribes to the binding
/// (`AppliedChangeObservable`).
///
/// A value the binding weighed and refused to write is not reported. Each change is delivered at
/// once to the observers subscribed when the delivery starts. The sequence never completes.
pub struct AppliedChangeObservable {
    this: Weak<AppliedChangeObservable>,
    observers: RefCell<Rc<[Rc<dyn IObserver<BindingChange>>]>>,
}

impl AppliedChangeObservable {
    /// A change stream with no observer.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), observers: RefCell::new(Rc::from(Vec::new())) })
    }

    /// Whether at least one observer is subscribed (`HasObservers`).
    pub fn has_observers(&self) -> bool {
        !self.observers.borrow().is_empty()
    }

    /// Reports a change the binding has written (`OnNext`).
    pub fn on_next(&self, value: BindingChange) {
        // The observers subscribed when the delivery starts; one that subscribes during it joins
        // the next change.
        let observers = self.observers.borrow().clone();
        for observer in observers.iter() {
            observer.on_next(value.clone());
        }
    }

    /// Drops one place of `observer` without disturbing a change being delivered; an observer that
    /// is not there is left alone (`Remove`).
    pub fn remove(&self, observer: &Rc<dyn IObserver<BindingChange>>) {
        let mut observers = self.observers.borrow_mut();
        let Some(index) = observers.iter().position(|candidate| std::ptr::addr_eq(Rc::as_ptr(candidate), Rc::as_ptr(observer))) else {
            return;
        };
        let mut remaining: Vec<Rc<dyn IObserver<BindingChange>>> = observers.to_vec();
        remaining.remove(index);
        *observers = Rc::from(remaining);
    }
}

impl IObservable<BindingChange> for AppliedChangeObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<BindingChange>>) -> Rc<dyn IDisposable> {
        {
            let mut observers = self.observers.borrow_mut();
            let mut added: Vec<Rc<dyn IObserver<BindingChange>>> = observers.to_vec();
            added.push(observer.clone());
            *observers = Rc::from(added);
        }
        Rc::new(Subscription { parent: self.this.clone(), observer: RefCell::new(Some(observer)) })
    }
}

impl fmt::Debug for AppliedChangeObservable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppliedChangeObservable: {} observer(s)", self.observers.borrow().len())
    }
}

/// The place of one observer in the change stream.
struct Subscription {
    parent: Weak<AppliedChangeObservable>,
    /// The observer to drop, cleared once dropped, so that disposing twice drops one place.
    observer: RefCell<Option<Rc<dyn IObserver<BindingChange>>>>,
}

impl IDisposable for Subscription {
    fn dispose(&self) {
        let Some(dropped) = self.observer.borrow_mut().take() else {
            return;
        };
        if let Some(parent) = self.parent.upgrade() {
            parent.remove(&dropped);
        }
    }
}
