//! `IsEmpty` (`Linq/Observable/IsEmpty.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::marker::PhantomData;
use std::rc::Rc;

/// The sink of `IsEmpty` (`IsEmpty<TSource>._`): the first value gives `false`; the end of the
/// sequence gives `true`.
struct IsEmptySink<T> {
    sink: Sink<bool>,
    source: PhantomData<fn(T)>,
}

impl<T: 'static> OperatorSink<T, bool> for IsEmptySink<T> {
    fn sink(&self) -> &Sink<bool> {
        &self.sink
    }
}

impl_sink_dispose!(IsEmptySink<T>);

impl<T: 'static> IObserver<T> for IsEmptySink<T> {
    fn on_next(&self, _value: T) {
        self.sink.forward_on_next(false);
        self.sink.forward_on_completed();
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(true);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn is_empty<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<bool>> {
    unary(source, |observer| Rc::new(IsEmptySink::<T> { sink: Sink::new(observer), source: PhantomData }))
}

/// The methods of `IsEmpty` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! is_empty_methods {
    () => {
        /// Whether the sequence ends without a value (`IsEmpty()`).
        fn is_empty(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::is_empty::is_empty(self.observable())
        }
    };
}
pub(crate) use is_empty_methods;
