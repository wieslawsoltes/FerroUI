//! `ToList` (`Linq/Observable/ToList.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The sink of `ToList` (`ToList<TSource>._`): collects the values and gives them at the end.
struct ToListSink<T> {
    sink: Sink<Vec<T>>,
    list: RefCell<Vec<T>>,
}

impl<T: 'static> OperatorSink<T, Vec<T>> for ToListSink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(ToListSink<T>);

impl<T: 'static> IObserver<T> for ToListSink<T> {
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

pub(crate) fn to_list<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<Vec<T>>> {
    unary(source, |observer| Rc::new(ToListSink { sink: Sink::new(observer), list: RefCell::new(Vec::new()) }))
}

/// The methods of `ToList` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! to_list_methods {
    () => {
        /// The values in a list, given when the sequence ends (`ToList()`).
        fn to_list(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::to_list::to_list(self.observable())
        }
    };
}
pub(crate) use to_list_methods;
