//! `AsObservable` (`Linq/Observable/AsObservable.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The sink of `AsObservable` (`AsObservable<TSource>._`, an `IdentitySink<TSource>`): forwards
/// every notification.
struct AsObservableSink<T> {
    sink: Sink<T>,
}

impl<T: 'static> OperatorSink<T, T> for AsObservableSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(AsObservableSink<T>);

impl<T: 'static> IObserver<T> for AsObservableSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn as_observable<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    // Deviation (DEVIATIONS.md, Reactive): upstream returns the source itself when it is already
    // the result of `AsObservable`; the handle of a sequence in the port does not reveal the type
    // of the sequence, so each call hides the source again (the notifications are the same).
    unary(source, |observer| Rc::new(AsObservableSink { sink: Sink::new(observer) }))
}

/// The methods of `AsObservable` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! as_observable_methods {
    () => {
        /// Hides the identity of the sequence: the result notifies what the source notifies, and
        /// subscribes to the source only when it is subscribed to (`AsObservable()`).
        fn as_observable(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::as_observable::as_observable(self.observable())
        }
    };
}
pub(crate) use as_observable_methods;
