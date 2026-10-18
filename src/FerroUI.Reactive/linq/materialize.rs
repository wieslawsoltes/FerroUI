//! `Materialize` (`Linq/Observable/Materialize.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use crate::notification::Notification;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The sink of `Materialize` (`Materialize<TSource>._`): each notification of the source becomes a
/// value; the end of the source is followed by the completion.
struct MaterializeSink<T> {
    sink: Sink<Notification<T>>,
}

impl<T: 'static> OperatorSink<T, Notification<T>> for MaterializeSink<T> {
    fn sink(&self) -> &Sink<Notification<T>> {
        &self.sink
    }
}

impl_sink_dispose!(MaterializeSink<T>);

impl<T: 'static> IObserver<T> for MaterializeSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(Notification::OnNext(value));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_next(Notification::OnError(error));
        self.sink.forward_on_completed();
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(Notification::OnCompleted);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn materialize<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<Notification<T>>> {
    unary(source, |observer| Rc::new(MaterializeSink { sink: Sink::new(observer) }))
}

/// The methods of `Materialize` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! materialize_methods {
    () => {
        /// The notifications of the sequence as values, followed by the completion
        /// (`Materialize()`).
        fn materialize(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::Notification<T>>> {
            crate::linq::materialize::materialize(self.observable())
        }
    };
}
pub(crate) use materialize_methods;
