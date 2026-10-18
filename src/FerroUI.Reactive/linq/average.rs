//! `Average` (`Linq/Observable/Average.cs`).

use super::aggregate::SequenceError;
use super::numeric::Averageable;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

/// The sink of `Average` (`AverageInt32._`, `AverageInt64._`, `AverageSingle._`,
/// `AverageDouble._` and their `Nullable` forms): adds up and counts the values and gives their
/// average at the end; a sum or a count that does not fit ends the sequence with
/// [`SequenceError::Overflow`], and a sequence without a counted value ends with
/// [`SequenceError::NoElements`] (with `None` for an `Option` sequence).
struct AverageSink<T: Averageable> {
    sink: Sink<T::Output>,
    sum: Cell<T::Sum>,
    count: Cell<i64>,
}

impl<T: Averageable> OperatorSink<T, T::Output> for AverageSink<T> {
    fn sink(&self) -> &Sink<T::Output> {
        &self.sink
    }
}

impl_sink_dispose!(AverageSink<T> where T: Averageable);

impl<T: Averageable> IObserver<T> for AverageSink<T> {
    fn on_next(&self, value: T) {
        match T::add(self.sum.get(), value) {
            Ok(Some(sum)) => match self.count.get().checked_add(1) {
                Some(count) => {
                    self.sum.set(sum);
                    self.count.set(count);
                }
                None => self.sink.forward_on_error(SequenceError::Overflow.into_error()),
            },
            Ok(None) => {}
            Err(error) => self.sink.forward_on_error(error.into_error()),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let count = self.count.get();
        let result = if count > 0 { Some(T::average(self.sum.get(), count)) } else { T::empty() };
        match result {
            Some(result) => {
                self.sink.forward_on_next(result);
                self.sink.forward_on_completed();
            }
            None => self.sink.forward_on_error(SequenceError::NoElements.into_error()),
        }
    }
}

pub(crate) fn average<T: Averageable>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T::Output>> {
    unary(source, |observer| Rc::new(AverageSink::<T> { sink: Sink::new(observer), sum: Cell::new(T::ZERO), count: Cell::new(0) }))
}

/// The methods of `Average` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! average_methods {
    () => {
        /// The average of the values (`Average()` of `int`, `long`, `float`, `double` and their
        /// nullable forms): an `f64` for `i32`, `i64` and `f64` values, an `f32` for `f32` values,
        /// an `Option` of it for an `Option` sequence (which ignores its `None` values); a sequence
        /// without a value ends with `SequenceError::NoElements`, an `Option` sequence without a
        /// `Some` gives `None`.
        fn average(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<<T as crate::linq::numeric::Averageable>::Output>>
        where
            T: crate::linq::numeric::Averageable,
        {
            crate::linq::average::average(self.observable())
        }

        /// The average of the values projected with `selector` (`Average(Func<TSource, int>)` and
        /// the overloads for `long`, `float`, `double` and their nullable forms).
        fn average_of<R: crate::linq::numeric::Averageable>(
            &self,
            selector: impl Fn(T) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<<R as crate::linq::numeric::Averageable>::Output>> {
            crate::linq::average::average(crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value)))))
        }
    };
}
pub(crate) use average_methods;
