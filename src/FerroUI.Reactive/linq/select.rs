//! `Select` (`Linq/Observable/Select.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

type Selector<T, U> = Rc<dyn Fn(T, usize) -> Result<U, ObservableError>>;

/// The sink of `Select` (`Select<TSource, TResult>.Selector._` and `SelectorIndexed._`): a
/// selector that fails ends the sequence with its error.
struct SelectSink<T, U> {
    sink: Sink<U>,
    selector: Selector<T, U>,
    index: Cell<usize>,
}

impl<T: 'static, U: 'static> OperatorSink<T, U> for SelectSink<T, U> {
    fn sink(&self) -> &Sink<U> {
        &self.sink
    }
}

impl_sink_dispose!(SelectSink<T, U>);

impl<T: 'static, U: 'static> IObserver<T> for SelectSink<T, U> {
    fn on_next(&self, value: T) {
        let index = self.index.get();
        self.index.set(index + 1);
        match (self.selector)(value, index) {
            Ok(result) => self.sink.forward_on_next(result),
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn select<T: 'static, U: 'static>(source: Rc<dyn IObservable<T>>, selector: Selector<T, U>) -> Rc<dyn IObservable<U>> {
    unary(source, move |observer| Rc::new(SelectSink { sink: Sink::new(observer), selector: selector.clone(), index: Cell::new(0) }))
}

/// The methods of `Select` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! select_methods {
    () => {
        /// Projects each value with `selector` (`Select(Func<TSource, TResult>)`).
        fn select<U: 'static>(&self, selector: impl Fn(T) -> U + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value))))
        }

        /// Projects each value with its index (`Select(Func<TSource, int, TResult>)`).
        fn select_with_index<U: 'static>(&self, selector: impl Fn(T, usize) -> U + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, index| Ok(selector(value, index))))
        }

        /// Projects each value with a selector that may fail; a failure ends the sequence with its
        /// error (`Select` with a selector that throws).
        fn try_select<U: 'static>(
            &self,
            selector: impl Fn(T) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| selector(value)))
        }

        /// [`try_select`](Self::try_select) with the index of the value.
        fn try_select_with_index<U: 'static>(
            &self,
            selector: impl Fn(T, usize) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select::select(self.observable(), std::rc::Rc::new(selector))
        }
    };
}
pub(crate) use select_methods;
