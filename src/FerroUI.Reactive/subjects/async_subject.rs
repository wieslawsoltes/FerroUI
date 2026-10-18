//! `AsyncSubject<T>` (`AsyncSubject.cs`).

use super::subject::{throw_disposed, ObserverList, SubjectBase, SubjectSubscription};
use crate::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A subject that keeps the last value and hands it, with the end, to every observer once the
/// sequence has ended (`AsyncSubject<T>`).
pub struct AsyncSubject<T> {
    this: Weak<AsyncSubject<T>>,
    observers: ObserverList<T>,
    inner: RefCell<Inner<T>>,
}

#[derive(Clone)]
enum Phase {
    Active,
    Terminated,
    Disposed,
}

struct Inner<T> {
    phase: Phase,
    value: Option<T>,
    error: Option<ObservableError>,
}

impl<T: Clone + 'static> AsyncSubject<T> {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            observers: ObserverList::new(),
            inner: RefCell::new(Inner { phase: Phase::Active, value: None, error: None }),
        })
    }

    /// Whether the subject has ended (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        matches!(self.inner.borrow().phase, Phase::Terminated)
    }

    /// The last value of an ended subject (`GetResult`): the error when it ended with one, `None`
    /// when it ended without a value (upstream throws `InvalidOperationException`). Panics when the
    /// subject has not ended, where upstream blocks until it does (a wait on the only thread would
    /// never end).
    pub fn get_result(&self) -> Result<Option<T>, ObservableError> {
        let inner = self.inner.borrow();
        match inner.phase {
            Phase::Disposed => throw_disposed(),
            Phase::Active => panic!("the subject has not completed; on a single thread waiting for it would never end"),
            Phase::Terminated => match &inner.error {
                Some(error) => Err(error.clone()),
                None => Ok(inner.value.clone()),
            },
        }
    }

    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }

    pub fn as_observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }
}

impl<T: Clone + 'static> IObserver<T> for AsyncSubject<T> {
    fn on_next(&self, value: T) {
        let mut inner = self.inner.borrow_mut();
        match inner.phase {
            Phase::Disposed => throw_disposed(),
            Phase::Terminated => {}
            Phase::Active => inner.value = Some(value),
        }
    }

    fn on_error(&self, error: ObservableError) {
        {
            let mut inner = self.inner.borrow_mut();
            match inner.phase {
                Phase::Disposed => throw_disposed(),
                Phase::Terminated => return,
                Phase::Active => {}
            }
            inner.phase = Phase::Terminated;
            inner.value = None;
            inner.error = Some(error.clone());
        }
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_error(error.clone());
            }
        }
    }

    fn on_completed(&self) {
        let value = {
            let mut inner = self.inner.borrow_mut();
            match inner.phase {
                Phase::Disposed => throw_disposed(),
                Phase::Terminated => return,
                Phase::Active => {}
            }
            inner.phase = Phase::Terminated;
            inner.value.clone()
        };
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                if let Some(value) = &value {
                    observer.on_next(value.clone());
                }
                observer.on_completed();
            }
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for AsyncSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (phase, value, error) = {
            let inner = self.inner.borrow();
            (inner.phase.clone(), inner.value.clone(), inner.error.clone())
        };
        match phase {
            Phase::Disposed => throw_disposed(),
            Phase::Terminated => {
                match error {
                    Some(error) => observer.on_error(error),
                    None => {
                        if let Some(value) = value {
                            observer.on_next(value);
                        }
                        observer.on_completed();
                    }
                }
                Disposable::empty()
            }
            Phase::Active => {
                let subject = self.this.clone();
                let subscription = SubjectSubscription::new(observer, move |subscription| {
                    if let Some(subject) = subject.upgrade() {
                        subject.observers.remove(subscription);
                    }
                });
                self.observers.add(subscription.clone());
                subscription
            }
        }
    }
}

impl<T: Clone + 'static> IDisposable for AsyncSubject<T> {
    fn dispose(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.phase = Phase::Disposed;
            inner.value = None;
            inner.error = None;
        }
        self.observers.take();
    }
}

impl<T: Clone + 'static> SubjectBase<T> for AsyncSubject<T> {
    fn has_observers(&self) -> bool {
        self.observers.len() != 0
    }

    fn is_disposed(&self) -> bool {
        matches!(self.inner.borrow().phase, Phase::Disposed)
    }
}
