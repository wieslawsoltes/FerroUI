//! `All` (`Linq/Observable/All.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T) -> Result<bool, ObservableError>>;

/// The sink of `All` (`All<TSource>._`): the first value the predicate is false for gives `false`;
/// the end of the sequence gives `true`.
struct AllSink<T> {
    sink: Sink<bool>,
    predicate: Predicate<T>,
}

impl<T: 'static> OperatorSink<T, bool> for AllSink<T> {
    fn sink(&self) -> &Sink<bool> {
        &self.sink
    }
}

impl_sink_dispose!(AllSink<T>);

impl<T: 'static> IObserver<T> for AllSink<T> {
    fn on_next(&self, value: T) {
        match (self.predicate)(&value) {
            Ok(true) => {}
            Ok(false) => {
                self.sink.forward_on_next(false);
                self.sink.forward_on_completed();
            }
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(true);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn all<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<bool>> {
    unary(source, move |observer| Rc::new(AllSink { sink: Sink::new(observer), predicate: predicate.clone() }))
}

/// The methods of `All` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! all_methods {
    () => {
        /// Whether `predicate` is true for every value (`All(Func<TSource, bool>)`).
        fn all(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::all::all(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`all`](Self::all) with a predicate that may fail; a failure ends the sequence with its
        /// error (`All` with a predicate that throws).
        fn try_all(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::all::all(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use all_methods;
