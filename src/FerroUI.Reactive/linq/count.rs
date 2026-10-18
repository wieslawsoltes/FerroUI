//! `Count` (`Linq/Observable/Count.cs`) and `LongCount` (`Linq/Observable/LongCount.cs`).

use super::aggregate::SequenceError;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T) -> Result<bool, ObservableError>>;

/// The type a count is given in: `usize` for `Count` (upstream's `int`), `u64` for `LongCount`
/// (upstream's `long`).
pub(crate) trait Counter: Copy + 'static {
    /// The count of an empty sequence.
    const ZERO: Self;

    /// The count plus one, or `None` when it does not fit (the `checked` increment of upstream).
    fn checked_increment(self) -> Option<Self>;
}

impl Counter for usize {
    const ZERO: Self = 0;

    fn checked_increment(self) -> Option<Self> {
        self.checked_add(1)
    }
}

impl Counter for u64 {
    const ZERO: Self = 0;

    fn checked_increment(self) -> Option<Self> {
        self.checked_add(1)
    }
}

/// The sink of `Count` and `LongCount` (`Count<TSource>.All._` and `LongCount<TSource>.All._`
/// without a predicate, `Count<TSource>.Predicate._` and `LongCount<TSource>.Predicate._` with
/// one): counts the values (the predicate is true for) and gives the count at the end; a count that
/// overflows ends the sequence with [`SequenceError::Overflow`].
struct CountSink<T, N> {
    sink: Sink<N>,
    predicate: Option<Predicate<T>>,
    count: Cell<N>,
}

impl<T: 'static, N: Counter> OperatorSink<T, N> for CountSink<T, N> {
    fn sink(&self) -> &Sink<N> {
        &self.sink
    }
}

impl_sink_dispose!(CountSink<T, N> where N: Counter);

impl<T: 'static, N: Counter> IObserver<T> for CountSink<T, N> {
    fn on_next(&self, value: T) {
        let counted = match &self.predicate {
            None => Ok(true),
            Some(predicate) => predicate(&value),
        };
        match counted {
            Ok(true) => match self.count.get().checked_increment() {
                Some(count) => self.count.set(count),
                None => self.sink.forward_on_error(SequenceError::Overflow.into_error()),
            },
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(self.count.get());
        self.sink.forward_on_completed();
    }
}

fn count_sinks<T: 'static, N: Counter>(source: Rc<dyn IObservable<T>>, predicate: Option<Predicate<T>>) -> Rc<dyn IObservable<N>> {
    unary(source, move |observer| Rc::new(CountSink { sink: Sink::new(observer), predicate: predicate.clone(), count: Cell::new(N::ZERO) }))
}

pub(crate) fn count<T: 'static, N: Counter>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<N>> {
    count_sinks(source, None)
}

pub(crate) fn count_with<T: 'static, N: Counter>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<N>> {
    count_sinks(source, Some(predicate))
}

/// The methods of `Count` and `LongCount` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! count_methods {
    () => {
        /// The number of values (`Count()`).
        fn count(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<usize>> {
            crate::linq::count::count(self.observable())
        }

        /// The number of values `predicate` is true for (`Count(Func<TSource, bool>)`).
        fn count_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<usize>> {
            crate::linq::count::count_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`count_with`](Self::count_with) with a predicate that may fail; a failure ends the
        /// sequence with its error (`Count(Func<TSource, bool>)` with a predicate that throws).
        fn try_count_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<usize>> {
            crate::linq::count::count_with(self.observable(), std::rc::Rc::new(predicate))
        }

        /// The number of values as a `u64` (`LongCount()`).
        fn long_count(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<u64>> {
            crate::linq::count::count(self.observable())
        }

        /// The number of values `predicate` is true for, as a `u64` (`LongCount(Func<TSource, bool>)`).
        fn long_count_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<u64>> {
            crate::linq::count::count_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`long_count_with`](Self::long_count_with) with a predicate that may fail; a failure
        /// ends the sequence with its error (`LongCount(Func<TSource, bool>)` with a predicate that
        /// throws).
        fn try_long_count_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<u64>> {
            crate::linq::count::count_with(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use count_methods;
