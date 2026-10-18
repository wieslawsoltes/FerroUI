//! `Sum` (`Linq/Observable/Sum.cs`).

use super::aggregate::SequenceError;
use super::numeric::Summable;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

/// The sink of `Sum` (`SumInt32._`, `SumInt64._`, `SumSingle._`, `SumDouble._` and their
/// `Nullable` forms): adds up the values and gives the sum at the end; a sum that does not fit
/// ends the sequence with [`SequenceError::Overflow`].
struct SumSink<T: Summable> {
    sink: Sink<T>,
    sum: Cell<T::Sum>,
}

impl<T: Summable> OperatorSink<T, T> for SumSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(SumSink<T> where T: Summable);

impl<T: Summable> IObserver<T> for SumSink<T> {
    fn on_next(&self, value: T) {
        match T::add(self.sum.get(), value) {
            Some(sum) => self.sum.set(sum),
            None => self.sink.forward_on_error(SequenceError::Overflow.into_error()),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(T::total(self.sum.get()));
        self.sink.forward_on_completed();
    }
}

pub(crate) fn sum<T: Summable>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    unary(source, |observer| Rc::new(SumSink::<T> { sink: Sink::new(observer), sum: Cell::new(T::ZERO) }))
}

/// The methods of `Sum` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! sum_methods {
    () => {
        /// The sum of the values (`Sum()` of `int`, `long`, `float`, `double` and their nullable
        /// forms): an `Option` sequence ignores its `None` values and gives `Some(0)` for no
        /// values; an `i32` or `i64` sum that does not fit ends the sequence with
        /// `SequenceError::Overflow` (`OverflowException`).
        fn sum(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: crate::linq::numeric::Summable,
        {
            crate::linq::sum::sum(self.observable())
        }

        /// The sum of the values projected with `selector` (`Sum(Func<TSource, int>)` and the
        /// overloads for `long`, `float`, `double` and their nullable forms).
        fn sum_of<R: crate::linq::numeric::Summable>(&self, selector: impl Fn(T) -> R + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::sum::sum(crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value)))))
        }
    };
}
pub(crate) use sum_methods;
