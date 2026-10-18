//! `Aggregate` (`Linq/Observable/Aggregate.cs`), and [`SequenceError`], the errors the aggregate
//! operators end a sequence with.

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The error an aggregate operator ends its sequence with when the sequence does not have the
/// shape the operator needs (the exceptions upstream's operators throw into `OnError`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SequenceError {
    /// The sequence has no elements (`InvalidOperationException` with `Strings_Linq.NO_ELEMENTS`).
    NoElements,
    /// No element of the sequence satisfies the predicate (`InvalidOperationException` with
    /// `Strings_Linq.NO_MATCHING_ELEMENTS`).
    NoMatchingElements,
    /// The sequence has more than one element (`InvalidOperationException` with
    /// `Strings_Linq.MORE_THAN_ONE_ELEMENT`).
    MoreThanOneElement,
    /// More than one element satisfies the predicate (`InvalidOperationException` with
    /// `Strings_Linq.MORE_THAN_ONE_MATCHING_ELEMENT`).
    MoreThanOneMatchingElement,
    /// A count does not fit its type (`OverflowException` of the `checked` counts).
    Overflow,
    /// Two elements have the same key (`ArgumentException` of `Dictionary<TKey, TValue>.Add`).
    DuplicateKey,
}

impl SequenceError {
    /// The error as the value an observer receives.
    pub(crate) fn into_error(self) -> ObservableError {
        Rc::new(self)
    }

    /// Whether the error is one of upstream's `InvalidOperationException`s (the sequence has no
    /// element, no matching element, or more than one).
    pub fn is_invalid_operation(&self) -> bool {
        matches!(self, Self::NoElements | Self::NoMatchingElements | Self::MoreThanOneElement | Self::MoreThanOneMatchingElement)
    }
}

impl fmt::Display for SequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoElements => "Sequence contains no elements.",
            Self::NoMatchingElements => "Sequence contains no matching element.",
            Self::MoreThanOneElement => "Sequence contains more than one element.",
            Self::MoreThanOneMatchingElement => "Sequence contains more than one matching element.",
            Self::Overflow => "Arithmetic operation resulted in an overflow.",
            Self::DuplicateKey => "An item with the same key has already been added.",
        })
    }
}

impl std::error::Error for SequenceError {}

type Accumulator<T> = Rc<dyn Fn(T, T) -> Result<T, ObservableError>>;
type SeedAccumulator<T, A> = Rc<dyn Fn(A, T) -> Result<A, ObservableError>>;
type ResultSelector<A, R> = Rc<dyn Fn(A) -> Result<R, ObservableError>>;

/// The sink of `Aggregate` without a seed (`Aggregate<TSource>._`): the first value is the
/// accumulation; an empty sequence ends with [`SequenceError::NoElements`].
struct AggregateSink<T> {
    sink: Sink<T>,
    accumulator: Accumulator<T>,
    accumulation: RefCell<Option<T>>,
    has_accumulation: Cell<bool>,
}

impl<T: 'static> OperatorSink<T, T> for AggregateSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(AggregateSink<T>);

impl<T: 'static> IObserver<T> for AggregateSink<T> {
    fn on_next(&self, value: T) {
        if !self.has_accumulation.get() {
            *self.accumulation.borrow_mut() = Some(value);
            self.has_accumulation.set(true);
            return;
        }
        let Some(accumulation) = self.accumulation.take() else {
            return;
        };
        match (self.accumulator)(accumulation, value) {
            Ok(next) => {
                *self.accumulation.borrow_mut() = Some(next);
            }
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        drop(self.accumulation.take());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        if !self.has_accumulation.get() {
            self.sink.forward_on_error(SequenceError::NoElements.into_error());
            return;
        }
        if let Some(accumulation) = self.accumulation.take() {
            self.sink.forward_on_next(accumulation);
            self.sink.forward_on_completed();
        }
    }
}

/// The sink of `Aggregate` with a seed (`Aggregate<TSource, TAccumulate>._` and
/// `Aggregate<TSource, TAccumulate, TResult>._`; the form without a result selector selects the
/// accumulation itself).
struct AggregateSeedSink<T, A, R> {
    sink: Sink<R>,
    accumulator: SeedAccumulator<T, A>,
    result_selector: ResultSelector<A, R>,
    accumulation: RefCell<Option<A>>,
}

impl<T: 'static, A: 'static, R: 'static> OperatorSink<T, R> for AggregateSeedSink<T, A, R> {
    fn sink(&self) -> &Sink<R> {
        &self.sink
    }
}

impl_sink_dispose!(AggregateSeedSink<T, A, R>);

impl<T: 'static, A: 'static, R: 'static> IObserver<T> for AggregateSeedSink<T, A, R> {
    fn on_next(&self, value: T) {
        let Some(accumulation) = self.accumulation.take() else {
            return;
        };
        match (self.accumulator)(accumulation, value) {
            Ok(next) => {
                *self.accumulation.borrow_mut() = Some(next);
            }
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        drop(self.accumulation.take());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let Some(accumulation) = self.accumulation.take() else {
            return;
        };
        match (self.result_selector)(accumulation) {
            Ok(result) => {
                self.sink.forward_on_next(result);
                self.sink.forward_on_completed();
            }
            Err(error) => self.sink.forward_on_error(error),
        }
    }
}

pub(crate) fn aggregate<T: 'static>(source: Rc<dyn IObservable<T>>, accumulator: Accumulator<T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(AggregateSink {
            sink: Sink::new(observer),
            accumulator: accumulator.clone(),
            accumulation: RefCell::new(None),
            has_accumulation: Cell::new(false),
        })
    })
}

pub(crate) fn aggregate_with_seed<T: 'static, A: Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    seed: A,
    accumulator: SeedAccumulator<T, A>,
    result_selector: ResultSelector<A, R>,
) -> Rc<dyn IObservable<R>> {
    unary(source, move |observer| {
        Rc::new(AggregateSeedSink {
            sink: Sink::new(observer),
            accumulator: accumulator.clone(),
            result_selector: result_selector.clone(),
            accumulation: RefCell::new(Some(seed.clone())),
        })
    })
}

/// The methods of `Aggregate` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! aggregate_methods {
    () => {
        /// Accumulates the values with `accumulator`, the first value being the initial
        /// accumulation, and gives the final accumulation; an empty sequence ends with
        /// `SequenceError::NoElements`
        /// (`Aggregate(Func<TSource, TSource, TSource>)`).
        fn aggregate(&self, accumulator: impl Fn(T, T) -> T + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::aggregate::aggregate(self.observable(), std::rc::Rc::new(move |accumulation: T, value: T| Ok(accumulator(accumulation, value))))
        }

        /// [`aggregate`](Self::aggregate) with an accumulator that may fail; a failure ends the
        /// sequence with its error (`Aggregate` with an accumulator that throws).
        fn try_aggregate(
            &self,
            accumulator: impl Fn(T, T) -> Result<T, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::aggregate::aggregate(self.observable(), std::rc::Rc::new(accumulator))
        }

        /// Accumulates the values with `accumulator` from `seed`, and gives the final accumulation
        /// (`Aggregate(TAccumulate, Func<TAccumulate, TSource, TAccumulate>)`).
        fn aggregate_with_seed<A: Clone + 'static>(
            &self,
            seed: A,
            accumulator: impl Fn(A, T) -> A + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> {
            crate::linq::aggregate::aggregate_with_seed(
                self.observable(),
                seed,
                std::rc::Rc::new(move |accumulation: A, value: T| Ok(accumulator(accumulation, value))),
                std::rc::Rc::new(|accumulation: A| Ok(accumulation)),
            )
        }

        /// [`aggregate_with_seed`](Self::aggregate_with_seed) with an accumulator that may fail
        /// (`Aggregate(TAccumulate, Func<TAccumulate, TSource, TAccumulate>)` with an accumulator
        /// that throws).
        fn try_aggregate_with_seed<A: Clone + 'static>(
            &self,
            seed: A,
            accumulator: impl Fn(A, T) -> Result<A, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> {
            crate::linq::aggregate::aggregate_with_seed(self.observable(), seed, std::rc::Rc::new(accumulator), std::rc::Rc::new(|accumulation: A| Ok(accumulation)))
        }

        /// Accumulates the values with `accumulator` from `seed`, and gives the final accumulation
        /// projected with `result_selector`
        /// (`Aggregate(TAccumulate, Func<TAccumulate, TSource, TAccumulate>, Func<TAccumulate, TResult>)`).
        fn aggregate_with_seed_and_result<A: Clone + 'static, R: 'static>(
            &self,
            seed: A,
            accumulator: impl Fn(A, T) -> A + 'static,
            result_selector: impl Fn(A) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::aggregate::aggregate_with_seed(
                self.observable(),
                seed,
                std::rc::Rc::new(move |accumulation: A, value: T| Ok(accumulator(accumulation, value))),
                std::rc::Rc::new(move |accumulation: A| Ok(result_selector(accumulation))),
            )
        }

        /// [`aggregate_with_seed_and_result`](Self::aggregate_with_seed_and_result) with an
        /// accumulator and a result selector that may fail; a failure ends the sequence with its
        /// error.
        fn try_aggregate_with_seed_and_result<A: Clone + 'static, R: 'static>(
            &self,
            seed: A,
            accumulator: impl Fn(A, T) -> Result<A, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(A) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::aggregate::aggregate_with_seed(self.observable(), seed, std::rc::Rc::new(accumulator), std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use aggregate_methods;
