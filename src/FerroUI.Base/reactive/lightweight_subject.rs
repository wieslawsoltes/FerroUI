use super::{Disposable, IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct Inner<T> {
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<T>>)>>,
    next_id: Cell<u64>,
    completed: Cell<bool>,
}

/// A subject: both an observable and an observer that forwards everything it
/// receives to its current subscribers.
///
/// Subscribers may subscribe or unsubscribe from within a notification; the
/// change takes effect for the next notification.
pub struct LightweightSubject<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Default for LightweightSubject<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for LightweightSubject<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T> LightweightSubject<T> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                observers: RefCell::new(Vec::new()),
                next_id: Cell::new(0),
                completed: Cell::new(false),
            }),
        }
    }

    pub fn has_observers(&self) -> bool {
        !self.inner.observers.borrow().is_empty()
    }

    fn snapshot(&self) -> Vec<Rc<dyn IObserver<T>>> {
        self.inner.observers.borrow().iter().map(|(_, o)| o.clone()).collect()
    }
}

impl<T: 'static> LightweightSubject<T> {
    fn subscribe_core(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        if self.inner.completed.get() {
            observer.on_completed();
            return Disposable::empty();
        }
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        self.inner.observers.borrow_mut().push((id, observer));
        let weak: Weak<Inner<T>> = Rc::downgrade(&self.inner);
        Disposable::create(move || {
            if let Some(inner) = weak.upgrade() {
                inner.observers.borrow_mut().retain(|(i, _)| *i != id);
            }
        })
    }
}

impl<T: Clone> IObserver<T> for LightweightSubject<T> {
    fn on_next(&self, value: T) {
        let count = self.inner.observers.borrow().len();
        match count {
            0 => {}
            1 => {
                let observer = self.inner.observers.borrow()[0].1.clone();
                observer.on_next(value);
            }
            _ => {
                for observer in self.snapshot() {
                    observer.on_next(value.clone());
                }
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.inner.completed.set(true);
        let observers = std::mem::take(&mut *self.inner.observers.borrow_mut());
        for (_, observer) in observers {
            observer.on_error(error.clone());
        }
    }

    fn on_completed(&self) {
        self.inner.completed.set(true);
        let observers = std::mem::take(&mut *self.inner.observers.borrow_mut());
        for (_, observer) in observers {
            observer.on_completed();
        }
    }
}

impl<T: 'static> IObservable<T> for LightweightSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.subscribe_core(observer)
    }
}
