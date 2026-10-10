//! The subject of the tests: an observable the test pushes values into.
//!
//! It stands for the subject of the reactive library the reference tests
//! use, with its ownership: a subscription holds the subject until it is
//! disposed, and a subject that completed lets go of its observers. The
//! state of the subject is shared, so that a test can track whether
//! anything still holds it ([`Subject::state`]).

use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The shared state of a [`Subject`].
pub struct SubjectState<T> {
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<T>>)>>,
    next_id: Cell<u64>,
    completed: Cell<bool>,
}

/// A handle to a subject; clones refer to the same subject.
pub struct Subject<T> {
    state: Rc<SubjectState<T>>,
}

impl<T> Clone for Subject<T> {
    fn clone(&self) -> Self {
        Self { state: self.state.clone() }
    }
}

impl<T: Clone + 'static> Subject<T> {
    pub fn new() -> Self {
        Self {
            state: Rc::new(SubjectState {
                observers: RefCell::new(Vec::new()),
                next_id: Cell::new(0),
                completed: Cell::new(false),
            }),
        }
    }

    /// The shared state: the subject is alive while a handle, an observable
    /// or a subscription of it is.
    pub fn state(&self) -> &Rc<SubjectState<T>> {
        &self.state
    }

    /// A handle to the subject with the given state.
    pub fn from_state(state: Rc<SubjectState<T>>) -> Self {
        Self { state }
    }

    /// The subject as an observable.
    pub fn observable(&self) -> Rc<dyn IObservable<T>> {
        Rc::new(self.clone())
    }

    fn snapshot(&self) -> Vec<Rc<dyn IObserver<T>>> {
        self.state.observers.borrow().iter().map(|(_, observer)| observer.clone()).collect()
    }

    pub fn on_next(&self, value: T) {
        for observer in self.snapshot() {
            observer.on_next(value.clone());
        }
    }

    pub fn on_completed(&self) {
        self.state.completed.set(true);
        let observers = self.snapshot();
        self.state.observers.borrow_mut().clear();
        for observer in observers {
            observer.on_completed();
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for Subject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        if self.state.completed.get() {
            observer.on_completed();
            return Disposable::empty();
        }
        let id = self.state.next_id.get();
        self.state.next_id.set(id + 1);
        self.state.observers.borrow_mut().push((id, observer));
        // The subscription holds the subject until it is disposed.
        let subject = RefCell::new(Some(self.state.clone()));
        Disposable::create(move || {
            if let Some(state) = subject.borrow_mut().take() {
                state.observers.borrow_mut().retain(|(subscription, _)| *subscription != id);
            }
        })
    }
}
