use super::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The base of observables that accept exactly one subscriber and hand
/// themselves out as the subscription.
///
/// The observable embeds this state, implements `IObservable` by calling
/// [`subscribe`](Self::subscribe) and `IDisposable` by calling
/// [`dispose`](Self::dispose), passing what has to happen when the
/// subscription starts and ends.
pub struct SingleSubscriberObservableBase<T> {
    error: RefCell<Option<ObservableError>>,
    observer: RefCell<Option<Rc<dyn IObserver<T>>>>,
    completed: Cell<bool>,
}

impl<T> Default for SingleSubscriberObservableBase<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SingleSubscriberObservableBase<T> {
    pub fn new() -> Self {
        Self { error: RefCell::new(None), observer: RefCell::new(None), completed: Cell::new(false) }
    }

    /// Registers the observer. `subscribed` runs once it is registered,
    /// unless the observable has already failed or completed, in which case
    /// the observer is told so instead.
    ///
    /// Panics when there already is a subscriber.
    pub fn subscribe(&self, observer: Rc<dyn IObserver<T>>, subscribed: impl FnOnce()) {
        if self.observer.borrow().is_some() {
            panic!("The observable can only be subscribed once.");
        }
        let error = self.error.borrow().clone();
        if let Some(error) = error {
            observer.on_error(error);
        } else if self.completed.get() {
            observer.on_completed();
        } else {
            *self.observer.borrow_mut() = Some(observer);
            subscribed();
        }
    }

    /// Ends the subscription: runs `unsubscribed` and forgets the observer.
    pub fn dispose(&self, unsubscribed: impl FnOnce()) {
        unsubscribed();
        let observer = self.observer.borrow_mut().take();
        drop(observer);
    }

    /// Whether an observer is currently subscribed.
    #[inline]
    pub fn has_observer(&self) -> bool {
        self.observer.borrow().is_some()
    }

    #[inline]
    fn current(&self) -> Option<Rc<dyn IObserver<T>>> {
        self.observer.borrow().clone()
    }

    #[inline]
    pub fn publish_next(&self, value: T) {
        if let Some(observer) = self.current() {
            observer.on_next(value);
        }
    }

    pub fn publish_completed(&self, unsubscribed: impl FnOnce()) {
        self.completed.set(true);
        if let Some(observer) = self.current() {
            observer.on_completed();
            unsubscribed();
            let observer = self.observer.borrow_mut().take();
            drop(observer);
        }
    }

    pub fn publish_error(&self, error: ObservableError, unsubscribed: impl FnOnce()) {
        *self.error.borrow_mut() = Some(error.clone());
        if let Some(observer) = self.current() {
            observer.on_error(error);
            unsubscribed();
            let observer = self.observer.borrow_mut().take();
            drop(observer);
        }
    }
}
