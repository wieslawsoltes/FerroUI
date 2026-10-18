//! `DefaultIfEmpty` (`Linq/Observable/DefaultIfEmpty.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The sink of `DefaultIfEmpty` (`DefaultIfEmpty<TSource>._`): forwards the values, and the default
/// value before the completion of a source without values.
struct DefaultIfEmptySink<T> {
    sink: Sink<T>,
    default_value: RefCell<Option<T>>,
    found: Cell<bool>,
}

impl<T: 'static> OperatorSink<T, T> for DefaultIfEmptySink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(DefaultIfEmptySink<T>);

impl<T: 'static> IObserver<T> for DefaultIfEmptySink<T> {
    fn on_next(&self, value: T) {
        self.found.set(true);
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        if !self.found.get() {
            let default_value = self.default_value.borrow_mut().take();
            if let Some(default_value) = default_value {
                self.sink.forward_on_next(default_value);
            }
        }
        self.sink.forward_on_completed();
    }
}

pub(crate) fn default_if_empty<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, default_value: T) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(DefaultIfEmptySink { sink: Sink::new(observer), default_value: RefCell::new(Some(default_value.clone())), found: Cell::new(false) })
    })
}

/// The methods of `DefaultIfEmpty` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! default_if_empty_methods {
    () => {
        /// The values of the sequence, or the default value of `T` when it completes without
        /// values (`DefaultIfEmpty()`).
        fn default_if_empty(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone + Default,
        {
            crate::linq::default_if_empty::default_if_empty(self.observable(), T::default())
        }

        /// The values of the sequence, or `default_value` when it completes without values
        /// (`DefaultIfEmpty(TSource)`).
        fn default_if_empty_with(&self, default_value: T) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::default_if_empty::default_if_empty(self.observable(), default_value)
        }
    };
}
pub(crate) use default_if_empty_methods;
