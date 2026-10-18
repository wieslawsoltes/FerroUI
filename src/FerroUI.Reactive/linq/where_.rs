//! `Where` (`Linq/Observable/Where.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T, usize) -> Result<bool, ObservableError>>;

/// The sink of `Where` (`Where<TSource>.Predicate._` and `PredicateIndexed._`): a predicate that
/// fails ends the sequence with its error.
struct WhereSink<T> {
    sink: Sink<T>,
    predicate: Predicate<T>,
    index: Cell<usize>,
}

impl<T: 'static> OperatorSink<T, T> for WhereSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(WhereSink<T>);

impl<T: 'static> IObserver<T> for WhereSink<T> {
    fn on_next(&self, value: T) {
        let index = self.index.get();
        self.index.set(index + 1);
        match (self.predicate)(&value, index) {
            Ok(true) => self.sink.forward_on_next(value),
            Ok(false) => {}
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

pub(crate) fn where_<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(WhereSink { sink: Sink::new(observer), predicate: predicate.clone(), index: Cell::new(0) }))
}

/// The methods of `Where` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! where_methods {
    () => {
        /// The values for which `predicate` is true (`Where(Func<TSource, bool>)`).
        fn where_(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::where_::where_(self.observable(), std::rc::Rc::new(move |value, _| Ok(predicate(value))))
        }

        /// The values for which `predicate` is true with their index (`Where(Func<TSource, int, bool>)`).
        fn where_with_index(&self, predicate: impl Fn(&T, usize) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::where_::where_(self.observable(), std::rc::Rc::new(move |value, index| Ok(predicate(value, index))))
        }

        /// The values for which a predicate that may fail is true; a failure ends the sequence
        /// with its error (`Where` with a predicate that throws).
        fn try_where(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::where_::where_(self.observable(), std::rc::Rc::new(move |value, _| predicate(value)))
        }

        /// [`try_where`](Self::try_where) with the index of the value.
        fn try_where_with_index(
            &self,
            predicate: impl Fn(&T, usize) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::where_::where_(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use where_methods;
