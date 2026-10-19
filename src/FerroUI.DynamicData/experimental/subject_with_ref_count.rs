//! `SubjectWithRefCount<T>` (`Experimental/SubjectWithRefCount.cs`).

use super::ISubjectWithRefCount;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{ISubject, Subject};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

/// A subject that counts its subscriptions: each subscription adds one, its disposal removes it
/// (`SubjectWithRefCount<T>`). The notifications go to the subject it wraps.
pub(crate) struct SubjectWithRefCount<T> {
    subject: Rc<dyn ISubject<T>>,
    ref_count: Rc<Cell<usize>>,
}

impl<T: Clone + 'static> SubjectWithRefCount<T> {
    /// A counting subject over `subject`, or over a new `Subject<T>` (`SubjectWithRefCount(ISubject<T>?
    /// subject = null)`).
    pub(crate) fn new(subject: Option<Rc<dyn ISubject<T>>>) -> Rc<Self> {
        let subject = subject.unwrap_or_else(|| {
            let subject: Rc<dyn ISubject<T>> = Subject::new();
            subject
        });
        Rc::new(Self { subject, ref_count: Rc::new(Cell::new(0)) })
    }
}

impl<T: 'static> ISubjectWithRefCount<T> for SubjectWithRefCount<T> {
    fn ref_count(&self) -> usize {
        self.ref_count.get()
    }
}

impl<T: 'static> IObserver<T> for SubjectWithRefCount<T> {
    fn on_next(&self, value: T) {
        self.subject.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.subject.on_error(error);
    }

    fn on_completed(&self) {
        self.subject.on_completed();
    }
}

impl<T: 'static> IObservable<T> for SubjectWithRefCount<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.ref_count.set(self.ref_count.get() + 1);
        let subscriber = self.subject.subscribe(observer);
        let ref_count = self.ref_count.clone();
        Disposable::create(move || {
            ref_count.set(ref_count.get() - 1);
            subscriber.dispose();
        })
    }
}
