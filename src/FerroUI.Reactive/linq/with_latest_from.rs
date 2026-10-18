//! `WithLatestFrom` (`Linq/Observable/WithLatestFrom.cs`).

use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ResultSelector<T, U, R> = Rc<dyn Fn(T, U) -> Result<R, ObservableError>>;

/// The sink of `WithLatestFrom` (`WithLatestFrom<TFirst, TSecond, TResult>._`): each value of the
/// first sequence is combined with the latest value of the second, once it has one; the end of the
/// second sequence only ends its subscription.
struct WithLatestFromSink<T, U, R> {
    sink: Sink<R>,
    result_selector: ResultSelector<T, U, R>,
    has_latest: Cell<bool>,
    latest: RefCell<Option<U>>,
    second_disposable: SingleAssignmentDisposableValue,
}

impl<T: 'static, U: Clone + 'static, R: 'static> WithLatestFromSink<T, U, R> {
    fn run(self: &Rc<Self>, first: &Rc<dyn IObservable<T>>, second: &Rc<dyn IObservable<U>>) {
        let first_observer = Rc::new(FirstObserver { parent: self.clone() });
        let second_observer = Rc::new(SecondObserver { parent: self.clone() });
        self.second_disposable.set_disposable(Some(second.subscribe(second_observer)));
        let upstream = first.subscribe(first_observer);
        self.sink.set_upstream(upstream);
    }
}

impl<T: 'static, U: 'static, R: 'static> IDisposable for WithLatestFromSink<T, U, R> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.second_disposable.dispose();
        }
    }
}

/// The observer of the first sequence (`FirstObserver`).
struct FirstObserver<T, U, R> {
    parent: Rc<WithLatestFromSink<T, U, R>>,
}

impl<T: 'static, U: Clone + 'static, R: 'static> IObserver<T> for FirstObserver<T, U, R> {
    fn on_next(&self, value: T) {
        let parent = &self.parent;
        if parent.has_latest.get() {
            let latest = parent.latest.borrow().clone().expect("a latest value is held once there is one");
            match (parent.result_selector)(value, latest) {
                Ok(result) => parent.sink.forward_on_next(result),
                Err(error) => parent.sink.forward_on_error(error),
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.parent.sink.forward_on_completed();
    }
}

/// The observer of the second sequence (`SecondObserver`).
struct SecondObserver<T, U, R> {
    parent: Rc<WithLatestFromSink<T, U, R>>,
}

impl<T: 'static, U: 'static, R: 'static> IObserver<U> for SecondObserver<T, U, R> {
    fn on_next(&self, value: U) {
        *self.parent.latest.borrow_mut() = Some(value);
        if !self.parent.has_latest.get() {
            self.parent.has_latest.set(true);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.parent.second_disposable.dispose();
    }
}

/// The observable of `WithLatestFrom` (`WithLatestFrom<TFirst, TSecond, TResult>`).
struct WithLatestFromObservable<T, U, R> {
    first: Rc<dyn IObservable<T>>,
    second: Rc<dyn IObservable<U>>,
    result_selector: ResultSelector<T, U, R>,
}

impl<T: 'static, U: Clone + 'static, R: 'static> IObservable<R> for WithLatestFromObservable<T, U, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let (first, second, result_selector) = (self.first.clone(), self.second.clone(), self.result_selector.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(WithLatestFromSink {
                    sink: Sink::new(observer),
                    result_selector,
                    has_latest: Cell::new(false),
                    latest: RefCell::new(None),
                    second_disposable: SingleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<WithLatestFromSink<T, U, R>>| sink.run(&first, &second),
        )
    }
}

pub(crate) fn with_latest_from<T: 'static, U: Clone + 'static, R: 'static>(
    first: Rc<dyn IObservable<T>>,
    second: Rc<dyn IObservable<U>>,
    result_selector: ResultSelector<T, U, R>,
) -> Rc<dyn IObservable<R>> {
    Rc::new(WithLatestFromObservable { first, second, result_selector })
}

/// The methods of `WithLatestFrom` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! with_latest_from_methods {
    () => {
        /// Combines each value of the sequence with the latest value of `second`, once `second` has
        /// one (`WithLatestFrom(IObservable<TFirst>, IObservable<TSecond>, Func<TFirst, TSecond, TResult>)`).
        fn with_latest_from<U: Clone + 'static, R: 'static>(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            result_selector: impl Fn(T, U) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::with_latest_from::with_latest_from(self.observable(), second, std::rc::Rc::new(move |first, latest| Ok(result_selector(first, latest))))
        }

        /// [`with_latest_from`](Self::with_latest_from) with a selector that may fail; a failure ends
        /// the sequence with its error (`WithLatestFrom` with a selector that throws).
        fn try_with_latest_from<U: Clone + 'static, R: 'static>(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            result_selector: impl Fn(T, U) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::with_latest_from::with_latest_from(self.observable(), second, std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use with_latest_from_methods;
