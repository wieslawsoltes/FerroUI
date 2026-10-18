//! The form of an operator of one source (`Producer<TTarget, TSink>` with a `Sink<TSource, TTarget>`):
//! an observable that creates a sink for each subscription and subscribes the sink to the source.

use super::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// A sink of an operator: an observer of the source that forwards to the observer downstream
/// through its [`Sink`].
pub(crate) trait OperatorSink<T, U>: IObserver<T> + IDisposable {
    /// The state the sink forwards through.
    fn sink(&self) -> &Sink<U>;
}

/// An operator of one source: each subscription creates a sink with `make` and subscribes it to the
/// source (`Sink<TSource, TTarget>.Run(source)`).
struct UnaryOperator<T, U, S> {
    source: Rc<dyn IObservable<T>>,
    make: Rc<dyn Fn(Rc<dyn IObserver<U>>) -> Rc<S>>,
}

impl<T: 'static, U: 'static, S: OperatorSink<T, U> + 'static> IObservable<U> for UnaryOperator<T, U, S> {
    fn subscribe(&self, observer: Rc<dyn IObserver<U>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let make = self.make.clone();
        subscribe_sink(observer, move |observer| make(observer), move |sink: &Rc<S>| {
            let upstream = source.subscribe(sink.clone());
            sink.sink().set_upstream(upstream);
        })
    }
}

/// The observable of an operator of one source whose sinks `make` creates.
pub(crate) fn unary<T: 'static, U: 'static, S: OperatorSink<T, U> + 'static>(
    source: Rc<dyn IObservable<T>>,
    make: impl Fn(Rc<dyn IObserver<U>>) -> Rc<S> + 'static,
) -> Rc<dyn IObservable<U>> {
    Rc::new(UnaryOperator { source, make: Rc::new(make) })
}

/// Implements `IDisposable` for a sink by disposing its [`Sink`] (`Sink<T>.Dispose`), and the
/// terminal notifications of the source by forwarding them (the default `OnError` and
/// `OnCompleted` of `Sink<TSource, TTarget>`) when the sink states `forward_terminals`.
macro_rules! impl_sink_dispose {
    ($sink:ident<$($param:ident),*> $(where $($bound:tt)*)?) => {
        impl<$($param: 'static),*> ferroui_system::IDisposable for $sink<$($param),*> $(where $($bound)*)? {
            fn dispose(&self) {
                $crate::internal::OperatorSink::sink(self).dispose();
            }
        }
    };
}
pub(crate) use impl_sink_dispose;
