use super::anonymous_observer_non_generic_helper::AnonymousObserverNonGenericHelper;
use super::{IObserver, ObservableError};

/// An observer built from closures.
///
/// An observer without a closure for errors rethrows an error it is given
/// (it panics with the message of the error); one without a closure for
/// completion ignores it.
pub struct AnonymousObserver<T> {
    on_next: Box<dyn Fn(T)>,
    on_error: Box<dyn Fn(ObservableError)>,
    on_completed: Box<dyn Fn()>,
}

impl<T> AnonymousObserver<T> {
    pub fn new(on_next: impl Fn(T) + 'static) -> Self {
        Self {
            on_next: Box::new(on_next),
            on_error: Box::new(AnonymousObserverNonGenericHelper::throws_on_error),
            on_completed: Box::new(AnonymousObserverNonGenericHelper::no_op_completed),
        }
    }

    pub fn new_with_error(on_next: impl Fn(T) + 'static, on_error: impl Fn(ObservableError) + 'static) -> Self {
        Self::new(on_next).with_error(on_error)
    }

    pub fn new_with_completed(on_next: impl Fn(T) + 'static, on_completed: impl Fn() + 'static) -> Self {
        Self::new(on_next).with_completed(on_completed)
    }

    pub fn new_with_error_and_completed(
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Self {
        Self::new(on_next).with_error(on_error).with_completed(on_completed)
    }

    pub fn with_error(mut self, on_error: impl Fn(ObservableError) + 'static) -> Self {
        self.on_error = Box::new(on_error);
        self
    }

    pub fn with_completed(mut self, on_completed: impl Fn() + 'static) -> Self {
        self.on_completed = Box::new(on_completed);
        self
    }
}

impl<T> IObserver<T> for AnonymousObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value)
    }

    fn on_error(&self, error: ObservableError) {
        (self.on_error)(error)
    }

    fn on_completed(&self) {
        (self.on_completed)()
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn error() -> ObservableError {
        Rc::new(std::fmt::Error)
    }

    #[test]
    fn notifications_reach_the_closures() {
        let next = Rc::new(Cell::new(0));
        let errors = Rc::new(Cell::new(0));
        let completed = Rc::new(Cell::new(0));
        let (n, e, c) = (next.clone(), errors.clone(), completed.clone());
        let observer = AnonymousObserver::new_with_error_and_completed(
            move |value: i32| n.set(n.get() + value),
            move |_| e.set(e.get() + 1),
            move || c.set(c.get() + 1),
        );

        observer.on_next(2);
        observer.on_next(3);
        observer.on_error(error());
        observer.on_completed();

        assert_eq!((5, 1, 1), (next.get(), errors.get(), completed.get()));
    }

    #[test]
    fn completion_is_ignored_without_a_closure() {
        AnonymousObserver::new(|_: i32| {}).on_completed();
        AnonymousObserver::new_with_error(|_: i32| {}, |_| {}).on_completed();
    }

    #[test]
    #[should_panic]
    fn an_error_is_rethrown_without_a_closure() {
        AnonymousObserver::new_with_completed(|_: i32| {}, || {}).on_error(error());
    }
}
