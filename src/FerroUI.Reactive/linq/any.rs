//! `Any` (`Linq/Observable/Any.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T) -> Result<bool, ObservableError>>;

/// The sink of `Any` (`Any<TSource>.Count._` without a predicate, `Any<TSource>.Predicate._` with
/// one): the first value (the predicate is true for) gives `true`; the end of the sequence gives
/// `false`.
struct AnySink<T> {
    sink: Sink<bool>,
    predicate: Option<Predicate<T>>,
}

impl<T: 'static> OperatorSink<T, bool> for AnySink<T> {
    fn sink(&self) -> &Sink<bool> {
        &self.sink
    }
}

impl_sink_dispose!(AnySink<T>);

impl<T: 'static> IObserver<T> for AnySink<T> {
    fn on_next(&self, value: T) {
        let found = match &self.predicate {
            None => Ok(true),
            Some(predicate) => predicate(&value),
        };
        match found {
            Ok(true) => {
                self.sink.forward_on_next(true);
                self.sink.forward_on_completed();
            }
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(false);
        self.sink.forward_on_completed();
    }
}

fn any_sinks<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Option<Predicate<T>>) -> Rc<dyn IObservable<bool>> {
    unary(source, move |observer| Rc::new(AnySink { sink: Sink::new(observer), predicate: predicate.clone() }))
}

pub(crate) fn any<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<bool>> {
    any_sinks(source, None)
}

pub(crate) fn any_with<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<bool>> {
    any_sinks(source, Some(predicate))
}

/// The methods of `Any` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! any_methods {
    () => {
        /// Whether the sequence has a value (`Any()`).
        fn any(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::any::any(self.observable())
        }

        /// Whether `predicate` is true for a value (`Any(Func<TSource, bool>)`).
        fn any_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::any::any_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`any_with`](Self::any_with) with a predicate that may fail; a failure ends the
        /// sequence with its error (`Any(Func<TSource, bool>)` with a predicate that throws).
        fn try_any_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::any::any_with(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use any_methods;
