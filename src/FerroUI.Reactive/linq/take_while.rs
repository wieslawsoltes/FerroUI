//! `TakeWhile` (`Linq/Observable/TakeWhile.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T, usize) -> Result<bool, ObservableError>>;

/// The sink of `TakeWhile` (`TakeWhile<TSource>.Predicate._` and `PredicateIndexed._`): the values
/// while the predicate holds; the first value for which it does not completes the sequence. A
/// predicate that fails ends the sequence with its error.
struct TakeWhileSink<T> {
    sink: Sink<T>,
    predicate: Predicate<T>,
    running: Cell<bool>,
    index: Cell<usize>,
}

impl<T: 'static> OperatorSink<T, T> for TakeWhileSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(TakeWhileSink<T>);

impl<T: 'static> IObserver<T> for TakeWhileSink<T> {
    fn on_next(&self, value: T) {
        if !self.running.get() {
            return;
        }
        let index = self.index.get();
        self.index.set(index + 1);
        match (self.predicate)(&value, index) {
            Ok(running) => self.running.set(running),
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        }
        if self.running.get() {
            self.sink.forward_on_next(value);
        } else {
            self.sink.forward_on_completed();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn take_while<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(TakeWhileSink { sink: Sink::new(observer), predicate: predicate.clone(), running: Cell::new(true), index: Cell::new(0) })
    })
}

/// The methods of `TakeWhile` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! take_while_methods {
    () => {
        /// The values while `predicate` is true (`TakeWhile(Func<TSource, bool>)`).
        fn take_while(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_while::take_while(self.observable(), std::rc::Rc::new(move |value, _| Ok(predicate(value))))
        }

        /// The values while `predicate` is true, with the index of the value
        /// (`TakeWhile(Func<TSource, int, bool>)`).
        fn take_while_with_index(&self, predicate: impl Fn(&T, usize) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_while::take_while(self.observable(), std::rc::Rc::new(move |value, index| Ok(predicate(value, index))))
        }

        /// [`take_while`](Self::take_while) with a predicate that may fail; a failure ends the
        /// sequence with its error (`TakeWhile` with a predicate that throws).
        fn try_take_while(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_while::take_while(self.observable(), std::rc::Rc::new(move |value, _| predicate(value)))
        }

        /// [`try_take_while`](Self::try_take_while) with the index of the value.
        fn try_take_while_with_index(
            &self,
            predicate: impl Fn(&T, usize) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_while::take_while(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use take_while_methods;
