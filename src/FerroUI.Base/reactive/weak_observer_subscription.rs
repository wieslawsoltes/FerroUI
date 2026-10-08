use super::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A subscription that holds its observer weakly and ends itself once the
/// observer is gone.
pub struct WeakObserverSubscription<T> {
    observer: Weak<dyn IObserver<T>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl<T: 'static> WeakObserverSubscription<T> {
    fn new(observer: &Rc<dyn IObserver<T>>) -> Self {
        Self { observer: Rc::downgrade(observer), subscription: RefCell::new(None) }
    }

    /// Subscribes `observer` to `observable` without keeping the observer
    /// alive.
    pub fn subscribe(observable: &dyn IObservable<T>, observer: &Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let subscription = Rc::new(WeakObserverSubscription::new(observer));
        let inner = observable.subscribe(subscription.clone());
        *subscription.subscription.borrow_mut() = Some(inner);
        subscription
    }

    fn try_get_observer(&self) -> Option<Rc<dyn IObserver<T>>> {
        if let Some(observer) = self.observer.upgrade() {
            return Some(observer);
        }

        // The observer has been collected; unsubscribe from the observable.
        self.dispose();
        None
    }
}

impl<T: 'static> IObserver<T> for WeakObserverSubscription<T> {
    fn on_completed(&self) {
        if let Some(observer) = self.try_get_observer() {
            observer.on_completed();
        }
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(observer) = self.try_get_observer() {
            observer.on_error(error);
        }
    }

    fn on_next(&self, value: T) {
        if let Some(observer) = self.try_get_observer() {
            observer.on_next(value);
        }
    }
}

impl<T: 'static> IDisposable for WeakObserverSubscription<T> {
    fn dispose(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;
    use crate::reactive::{AnonymousObserver, LightweightSubject};

    #[test]
    fn forwards_while_the_observer_is_alive_and_unsubscribes_once_it_is_gone() {
        let subject = LightweightSubject::<i32>::new();
        let received = Rc::new(RefCell::new(Vec::new()));
        let r = received.clone();
        let observer: Rc<dyn IObserver<i32>> = Rc::new(AnonymousObserver::new(move |value: i32| r.borrow_mut().push(value)));

        let _subscription = WeakObserverSubscription::subscribe(&subject, &observer);
        subject.on_next(1);
        assert_eq!(vec![1], *received.borrow());
        assert!(subject.has_observers());

        drop(observer);
        subject.on_next(2);
        assert_eq!(vec![1], *received.borrow());
        assert!(!subject.has_observers());
    }

    #[test]
    fn dispose_unsubscribes() {
        let subject = LightweightSubject::<i32>::new();
        let observer: Rc<dyn IObserver<i32>> = Rc::new(AnonymousObserver::new(|_: i32| {}));

        let subscription = WeakObserverSubscription::subscribe(&subject, &observer);
        assert!(subject.has_observers());

        subscription.dispose();
        assert!(!subject.has_observers());
    }
}
