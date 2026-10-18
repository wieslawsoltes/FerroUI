//! `BehaviorSubject<T>` (`BehaviorSubject.cs`).

use super::subject::{throw_disposed, ObserverList, SubjectBase, SubjectSubscription};
use crate::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A subject that keeps the latest value and hands it to each observer that subscribes, then
/// every later notification (`BehaviorSubject<T>`).
pub struct BehaviorSubject<T> {
    this: Weak<BehaviorSubject<T>>,
    observers: ObserverList<T>,
    inner: RefCell<Inner<T>>,
}

struct Inner<T> {
    value: Option<T>,
    is_stopped: bool,
    error: Option<ObservableError>,
    is_disposed: bool,
}

impl<T: Clone + 'static> BehaviorSubject<T> {
    /// A subject whose value is `value` until it receives one.
    pub fn new(value: T) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            observers: ObserverList::new(),
            inner: RefCell::new(Inner { value: Some(value), is_stopped: false, error: None, is_disposed: false }),
        })
    }

    /// The latest value (`Value`): the error when the subject ended with one, where upstream
    /// throws it. Panics when the subject has been disposed.
    pub fn value(&self) -> Result<T, ObservableError> {
        let inner = self.inner.borrow();
        if inner.is_disposed {
            throw_disposed();
        }
        if let Some(error) = &inner.error {
            return Err(error.clone());
        }
        Ok(inner.value.clone().expect("a subject that is not disposed has a value"))
    }

    /// The latest value, or `None` once the subject has been disposed (`TryGetValue`); the error
    /// when the subject ended with one.
    pub fn try_get_value(&self) -> Result<Option<T>, ObservableError> {
        let inner = self.inner.borrow();
        if inner.is_disposed {
            return Ok(None);
        }
        if let Some(error) = &inner.error {
            return Err(error.clone());
        }
        Ok(inner.value.clone())
    }

    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }

    pub fn as_observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }
}

impl<T: Clone + 'static> IObserver<T> for BehaviorSubject<T> {
    fn on_next(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            if inner.is_stopped {
                return;
            }
            inner.value = Some(value.clone());
        }
        for subscription in self.observers.snapshot().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_next(value.clone());
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            if inner.is_stopped {
                return;
            }
            inner.is_stopped = true;
            inner.error = Some(error.clone());
        }
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_error(error.clone());
            }
        }
    }

    fn on_completed(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            if inner.is_stopped {
                return;
            }
            inner.is_stopped = true;
        }
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_completed();
            }
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for BehaviorSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (is_stopped, error, value) = {
            let inner = self.inner.borrow();
            if inner.is_disposed {
                throw_disposed();
            }
            (inner.is_stopped, inner.error.clone(), inner.value.clone())
        };
        if !is_stopped {
            let subject = self.this.clone();
            let subscription = SubjectSubscription::new(observer.clone(), move |subscription| {
                if let Some(subject) = subject.upgrade() {
                    if !subject.inner.borrow().is_disposed {
                        subject.observers.remove(subscription);
                    }
                }
            });
            self.observers.add(subscription.clone());
            if let Some(value) = value {
                observer.on_next(value);
            }
            return subscription;
        }
        match error {
            Some(error) => observer.on_error(error),
            None => observer.on_completed(),
        }
        Disposable::empty()
    }
}

impl<T: Clone + 'static> IDisposable for BehaviorSubject<T> {
    fn dispose(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.is_disposed = true;
            inner.value = None;
            inner.error = None;
        }
        self.observers.take();
    }
}

impl<T: Clone + 'static> SubjectBase<T> for BehaviorSubject<T> {
    fn has_observers(&self) -> bool {
        !self.inner.borrow().is_disposed && self.observers.len() > 0
    }

    fn is_disposed(&self) -> bool {
        self.inner.borrow().is_disposed
    }
}
