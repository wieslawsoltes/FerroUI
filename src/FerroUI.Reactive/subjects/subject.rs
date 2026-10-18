//! `ISubject`, `SubjectBase<T>` and `Subject<T>` (`ISubject.cs`, `SubjectBase.cs`, `Subject.cs`).

use crate::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// An observer and an observable at once (`ISubject<TSource, TResult>`, and `ISubject<T>` when both
/// are the same type).
pub trait ISubject<TSource, TResult = TSource>: IObserver<TSource> + IObservable<TResult> {}

impl<S: IObserver<A> + IObservable<B> + ?Sized, A, B> ISubject<A, B> for S {}

/// What every subject of the crate offers (`SubjectBase<T>`).
pub trait SubjectBase<T>: ISubject<T> + IDisposable {
    /// Whether observers are subscribed (`HasObservers`).
    fn has_observers(&self) -> bool;

    /// Whether the subject has been disposed (`IsDisposed`).
    fn is_disposed(&self) -> bool;
}

pub(crate) fn throw_disposed() -> ! {
    panic!("Cannot access a disposed object.");
}

/// The subscription of an observer to a subject: the observer is forgotten when it is disposed, so
/// that a notification in progress skips it.
pub(crate) struct SubjectSubscription<T> {
    observer: RefCell<Option<Rc<dyn IObserver<T>>>>,
    unsubscribe: RefCell<Option<Box<dyn FnOnce(&Rc<SubjectSubscription<T>>)>>>,
    this: Weak<SubjectSubscription<T>>,
}

impl<T: 'static> SubjectSubscription<T> {
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>, unsubscribe: impl FnOnce(&Rc<SubjectSubscription<T>>) + 'static) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            observer: RefCell::new(Some(observer)),
            unsubscribe: RefCell::new(Some(Box::new(unsubscribe))),
            this: this.clone(),
        })
    }

    pub(crate) fn observer(&self) -> Option<Rc<dyn IObserver<T>>> {
        self.observer.borrow().clone()
    }
}

impl<T: 'static> IDisposable for SubjectSubscription<T> {
    fn dispose(&self) {
        if self.observer.borrow_mut().take().is_none() {
            return;
        }
        let unsubscribe = self.unsubscribe.borrow_mut().take();
        if let (Some(unsubscribe), Some(this)) = (unsubscribe, self.this.upgrade()) {
            unsubscribe(&this);
        }
    }
}

/// The observers of a subject, replaced as a whole when one subscribes or unsubscribes, so that a
/// notification goes to the observers of the moment it started (copy on write).
pub(crate) struct ObserverList<T> {
    observers: RefCell<Rc<Vec<Rc<SubjectSubscription<T>>>>>,
}

impl<T: 'static> ObserverList<T> {
    pub(crate) fn new() -> Self {
        Self { observers: RefCell::new(Rc::new(Vec::new())) }
    }

    pub(crate) fn snapshot(&self) -> Rc<Vec<Rc<SubjectSubscription<T>>>> {
        self.observers.borrow().clone()
    }

    pub(crate) fn len(&self) -> usize {
        self.observers.borrow().len()
    }

    pub(crate) fn add(&self, subscription: Rc<SubjectSubscription<T>>) {
        let mut observers = self.observers.borrow_mut();
        let mut next: Vec<_> = observers.iter().cloned().collect();
        next.push(subscription);
        *observers = Rc::new(next);
    }

    pub(crate) fn remove(&self, subscription: &Rc<SubjectSubscription<T>>) {
        let mut observers = self.observers.borrow_mut();
        if let Some(index) = observers.iter().position(|s| Rc::ptr_eq(s, subscription)) {
            let mut next: Vec<_> = observers.iter().cloned().collect();
            next.remove(index);
            *observers = Rc::new(next);
        }
    }

    pub(crate) fn take(&self) -> Rc<Vec<Rc<SubjectSubscription<T>>>> {
        std::mem::replace(&mut *self.observers.borrow_mut(), Rc::new(Vec::new()))
    }
}

#[derive(Clone)]
enum State {
    Active,
    Terminated(Option<ObservableError>),
    Disposed,
}

/// A subject that hands every notification to the observers subscribed at that moment
/// (`Subject<T>`). An observer that subscribes after the end receives the end at once. Using a
/// disposed subject panics, where upstream throws `ObjectDisposedException`.
pub struct Subject<T> {
    this: Weak<Subject<T>>,
    observers: ObserverList<T>,
    state: RefCell<State>,
}

impl<T: Clone + 'static> Subject<T> {
    /// A subject without observers.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), observers: ObserverList::new(), state: RefCell::new(State::Active) })
    }

    /// The subject as the handle of an observable sequence.
    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }

    /// The subject as an observer.
    pub fn as_observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }
}

impl<T: Clone + 'static> IObserver<T> for Subject<T> {
    fn on_next(&self, value: T) {
        if matches!(*self.state.borrow(), State::Disposed) {
            throw_disposed();
        }
        for subscription in self.observers.snapshot().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_next(value.clone());
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        match *self.state.borrow() {
            State::Disposed => throw_disposed(),
            State::Terminated(_) => return,
            State::Active => {}
        }
        *self.state.borrow_mut() = State::Terminated(Some(error.clone()));
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_error(error.clone());
            }
        }
    }

    fn on_completed(&self) {
        match *self.state.borrow() {
            State::Disposed => throw_disposed(),
            State::Terminated(_) => return,
            State::Active => {}
        }
        *self.state.borrow_mut() = State::Terminated(None);
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_completed();
            }
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for Subject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let state = self.state.borrow().clone();
        match state {
            State::Disposed => throw_disposed(),
            State::Terminated(Some(error)) => {
                observer.on_error(error);
                Disposable::empty()
            }
            State::Terminated(None) => {
                observer.on_completed();
                Disposable::empty()
            }
            State::Active => {
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

impl<T: Clone + 'static> IDisposable for Subject<T> {
    fn dispose(&self) {
        *self.state.borrow_mut() = State::Disposed;
        self.observers.take();
    }
}

impl<T: Clone + 'static> SubjectBase<T> for Subject<T> {
    fn has_observers(&self) -> bool {
        self.observers.len() != 0
    }

    fn is_disposed(&self) -> bool {
        matches!(*self.state.borrow(), State::Disposed)
    }
}
