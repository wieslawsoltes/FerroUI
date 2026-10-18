//! `IgnoreElements` (`Linq/Observable/IgnoreElements.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The sink of `IgnoreElements` (`IgnoreElements<TSource>._`): drops the values and forwards the
/// end of the sequence.
struct IgnoreElementsSink<T> {
    sink: Sink<T>,
}

impl<T: 'static> OperatorSink<T, T> for IgnoreElementsSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(IgnoreElementsSink<T>);

impl<T: 'static> IObserver<T> for IgnoreElementsSink<T> {
    fn on_next(&self, _value: T) {}

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn ignore_elements<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    // Deviation (DEVIATIONS.md, Reactive): upstream returns the source itself when it is already
    // the result of `IgnoreElements`; the handle of a sequence in the port does not reveal the type
    // of the sequence, so the operator is applied again (the notifications are the same).
    unary(source, |observer| Rc::new(IgnoreElementsSink { sink: Sink::new(observer) }))
}

/// The methods of `IgnoreElements` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! ignore_elements_methods {
    () => {
        /// Ignores the values and keeps only the end of the sequence, its completion or its error
        /// (`IgnoreElements()`).
        fn ignore_elements(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ignore_elements::ignore_elements(self.observable())
        }
    };
}
pub(crate) use ignore_elements_methods;
