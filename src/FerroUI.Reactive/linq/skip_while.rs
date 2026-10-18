//! `SkipWhile` (`Linq/Observable/SkipWhile.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T, usize) -> Result<bool, ObservableError>>;

/// The sink of `SkipWhile` (`SkipWhile<TSource>.Predicate._` and `PredicateIndexed._`): the
/// values are dropped while the predicate holds; from the first value for which it does not, the
/// predicate is released and every value is forwarded. A predicate that fails ends the sequence
/// with its error.
struct SkipWhileSink<T> {
    sink: Sink<T>,
    predicate: RefCell<Option<Predicate<T>>>,
    index: Cell<usize>,
}

impl<T: 'static> OperatorSink<T, T> for SkipWhileSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(SkipWhileSink<T>);

impl<T: 'static> IObserver<T> for SkipWhileSink<T> {
    fn on_next(&self, value: T) {
        let predicate = self.predicate.borrow().clone();
        if let Some(predicate) = predicate {
            let index = self.index.get();
            self.index.set(index + 1);
            let should_start = match predicate(&value, index) {
                Ok(skip) => !skip,
                Err(error) => {
                    self.sink.forward_on_error(error);
                    return;
                }
            };
            if should_start {
                *self.predicate.borrow_mut() = None;
                self.sink.forward_on_next(value);
            }
        } else {
            self.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn skip_while<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(SkipWhileSink { sink: Sink::new(observer), predicate: RefCell::new(Some(predicate.clone())), index: Cell::new(0) })
    })
}

/// The methods of `SkipWhile` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! skip_while_methods {
    () => {
        /// The values from the first one for which `predicate` is false
        /// (`SkipWhile(Func<TSource, bool>)`).
        fn skip_while(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_while::skip_while(self.observable(), std::rc::Rc::new(move |value, _| Ok(predicate(value))))
        }

        /// The values from the first one for which `predicate` is false, with the index of the
        /// value (`SkipWhile(Func<TSource, int, bool>)`).
        fn skip_while_with_index(&self, predicate: impl Fn(&T, usize) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_while::skip_while(self.observable(), std::rc::Rc::new(move |value, index| Ok(predicate(value, index))))
        }

        /// [`skip_while`](Self::skip_while) with a predicate that may fail; a failure ends the
        /// sequence with its error (`SkipWhile` with a predicate that throws).
        fn try_skip_while(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_while::skip_while(self.observable(), std::rc::Rc::new(move |value, _| predicate(value)))
        }

        /// [`try_skip_while`](Self::try_skip_while) with the index of the value.
        fn try_skip_while_with_index(
            &self,
            predicate: impl Fn(&T, usize) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_while::skip_while(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use skip_while_methods;
