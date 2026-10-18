//! `Contains` (`Linq/Observable/Contains.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

type Comparer<T> = Rc<dyn Fn(&T, &T) -> Result<bool, ObservableError>>;

/// The sink of `Contains` (`Contains<TSource>._`): the first value equal to the one looked for
/// gives `true`; the end of the sequence gives `false`.
struct ContainsSink<T> {
    sink: Sink<bool>,
    value: Rc<T>,
    comparer: Comparer<T>,
}

impl<T: 'static> OperatorSink<T, bool> for ContainsSink<T> {
    fn sink(&self) -> &Sink<bool> {
        &self.sink
    }
}

impl_sink_dispose!(ContainsSink<T>);

impl<T: 'static> IObserver<T> for ContainsSink<T> {
    fn on_next(&self, value: T) {
        match (self.comparer)(&value, &self.value) {
            Ok(true) => {
                self.sink.forward_on_next(true);
                self.sink.forward_on_completed();
            }
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(false);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn contains<T: 'static>(source: Rc<dyn IObservable<T>>, value: T, comparer: Comparer<T>) -> Rc<dyn IObservable<bool>> {
    let value = Rc::new(value);
    unary(source, move |observer| Rc::new(ContainsSink { sink: Sink::new(observer), value: value.clone(), comparer: comparer.clone() }))
}

/// The methods of `Contains` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! contains_methods {
    () => {
        /// Whether the sequence has a value equal to `value` (`Contains(TSource)`).
        fn contains(&self, value: T) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>>
        where
            T: PartialEq,
        {
            crate::linq::contains::contains(self.observable(), value, std::rc::Rc::new(|x: &T, y: &T| Ok(x == y)))
        }

        /// Whether the sequence has a value that `comparer` finds equal to `value`; the comparer
        /// takes a value of the sequence and `value` (`Contains(TSource, IEqualityComparer<TSource>)`).
        fn contains_with(&self, value: T, comparer: impl Fn(&T, &T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::contains::contains(self.observable(), value, std::rc::Rc::new(move |x: &T, y: &T| Ok(comparer(x, y))))
        }

        /// [`contains_with`](Self::contains_with) with a comparer that may fail; a failure ends the
        /// sequence with its error (`Contains` with a comparer that throws).
        fn try_contains_with(
            &self,
            value: T,
            comparer: impl Fn(&T, &T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::contains::contains(self.observable(), value, std::rc::Rc::new(comparer))
        }
    };
}
pub(crate) use contains_methods;
