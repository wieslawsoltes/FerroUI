//! `OfType` (`Linq/Observable/OfType.cs`).
//!
//! Upstream filters a sequence of `object`; the port filters a sequence of `Rc<dyn Any>` (see
//! [`AnyObject`]) to the values of one type, `Rc<U>`.

use super::cast::AnyObject;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::marker::PhantomData;
use std::rc::Rc;

/// The sink of `OfType` (`OfType<TSource, TResult>._`): forwards the values that are a `U`.
struct OfTypeSink<T, U> {
    sink: Sink<Rc<U>>,
    _source: PhantomData<fn(T)>,
}

impl<T: AnyObject, U: 'static> OperatorSink<T, Rc<U>> for OfTypeSink<T, U> {
    fn sink(&self) -> &Sink<Rc<U>> {
        &self.sink
    }
}

impl_sink_dispose!(OfTypeSink<T, U> where T: AnyObject);

impl<T: AnyObject, U: 'static> IObserver<T> for OfTypeSink<T, U> {
    fn on_next(&self, value: T) {
        if let Ok(result) = value.downcast_object::<U>() {
            self.sink.forward_on_next(result);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn of_type<T: AnyObject, U: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<Rc<U>>> {
    unary(source, |observer| Rc::new(OfTypeSink::<T, U> { sink: Sink::new(observer), _source: PhantomData }))
}

/// The methods of `OfType` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! of_type_methods {
    () => {
        /// The values of a sequence of `Rc<dyn Any>` that are a `U`, as `U` (`OfType<TResult>()`).
        fn of_type<U: 'static>(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<U>>>
        where
            T: crate::linq::cast::AnyObject,
        {
            crate::linq::of_type::of_type::<T, U>(self.observable())
        }
    };
}
pub(crate) use of_type_methods;
