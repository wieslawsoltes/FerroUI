//! `ToArray` (`Linq/Observable/ToArray.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The sink of `ToArray` (`ToArray<TSource>._`): collects the values and gives them at the end.
struct ToArraySink<T> {
    sink: Sink<Vec<T>>,
    list: RefCell<Vec<T>>,
}

impl<T: 'static> OperatorSink<T, Vec<T>> for ToArraySink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(ToArraySink<T>);

impl<T: 'static> IObserver<T> for ToArraySink<T> {
    fn on_next(&self, value: T) {
        self.list.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        drop(self.list.take());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let list = self.list.take();
        self.sink.forward_on_next(list);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn to_array<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<Vec<T>>> {
    unary(source, |observer| Rc::new(ToArraySink { sink: Sink::new(observer), list: RefCell::new(Vec::new()) }))
}

/// The methods of `ToArray` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! to_array_methods {
    () => {
        /// The values in a vector, given when the sequence ends (`ToArray()`).
        ///
        /// Deviation (DEVIATIONS.md, Reactive): upstream gives an array (`TSource[]`); the port
        /// gives a `Vec<T>`, as [`to_list`](Self::to_list) does.
        fn to_array(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::to_array::to_array(self.observable())
        }
    };
}
pub(crate) use to_array_methods;
