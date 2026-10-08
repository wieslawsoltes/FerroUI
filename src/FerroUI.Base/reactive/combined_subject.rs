use super::{IDisposable, IFerroSubject, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// A subject made of an observer, which takes its notifications, and an
/// observable, which takes its subscriptions.
pub struct CombinedSubject<T> {
    observer: Rc<dyn IObserver<T>>,
    observable: Rc<dyn IObservable<T>>,
}

impl<T> CombinedSubject<T> {
    pub fn new(observer: Rc<dyn IObserver<T>>, observable: Rc<dyn IObservable<T>>) -> Self {
        Self { observer, observable }
    }
}

impl<T> IObserver<T> for CombinedSubject<T> {
    fn on_completed(&self) {
        self.observer.on_completed()
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error)
    }

    fn on_next(&self, value: T) {
        self.observer.on_next(value)
    }
}

impl<T> IObservable<T> for CombinedSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.observable.subscribe(observer)
    }
}

impl<T> IFerroSubject<T> for CombinedSubject<T> {}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;
    use crate::reactive::{AnonymousObserver, Observable};
    use std::cell::RefCell;

    #[test]
    fn notifications_go_to_the_observer_and_subscriptions_to_the_observable() {
        let received = Rc::new(RefCell::new(Vec::new()));
        let r = received.clone();
        let observer: Rc<dyn IObserver<i32>> = Rc::new(AnonymousObserver::new(move |value: i32| r.borrow_mut().push(value)));
        let target = CombinedSubject::new(observer, Observable::single_value(7));

        target.on_next(1);
        target.on_completed();
        assert_eq!(vec![1], *received.borrow());

        let subscribed = Rc::new(RefCell::new(Vec::new()));
        let s = subscribed.clone();
        let subscription = target.subscribe(Rc::new(AnonymousObserver::new(move |value: i32| s.borrow_mut().push(value))));
        subscription.dispose();
        assert_eq!(vec![7], *subscribed.borrow());
    }
}
