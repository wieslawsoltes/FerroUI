//! `Scan` (`Linq/Observable/Scan.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

type Accumulator<A, T> = Rc<dyn Fn(A, T) -> Result<A, ObservableError>>;

/// The sink of `Scan` with a seed (`Scan<TSource, TAccumulate>._`): forwards each accumulation; an
/// accumulator that fails ends the sequence with its error.
struct ScanSeedSink<T, A> {
    sink: Sink<A>,
    accumulator: Accumulator<A, T>,
    accumulation: RefCell<Option<A>>,
}

impl<T: 'static, A: Clone + 'static> OperatorSink<T, A> for ScanSeedSink<T, A> {
    fn sink(&self) -> &Sink<A> {
        &self.sink
    }
}

impl_sink_dispose!(ScanSeedSink<T, A> where A: Clone);

impl<T: 'static, A: Clone + 'static> IObserver<T> for ScanSeedSink<T, A> {
    fn on_next(&self, value: T) {
        let accumulation = self.accumulation.borrow_mut().take().expect("the accumulation is set between values");
        match (self.accumulator)(accumulation, value) {
            Ok(accumulation) => {
                *self.accumulation.borrow_mut() = Some(accumulation.clone());
                self.sink.forward_on_next(accumulation);
            }
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

/// The sink of `Scan` without a seed (`Scan<TSource>._`): the first value is the first
/// accumulation.
struct ScanSink<T> {
    sink: Sink<T>,
    accumulator: Accumulator<T, T>,
    accumulation: RefCell<Option<T>>,
}

impl<T: Clone + 'static> OperatorSink<T, T> for ScanSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(ScanSink<T> where T: Clone);

impl<T: Clone + 'static> IObserver<T> for ScanSink<T> {
    fn on_next(&self, value: T) {
        let accumulation = self.accumulation.borrow_mut().take();
        let next = match accumulation {
            Some(accumulation) => (self.accumulator)(accumulation, value),
            None => Ok(value),
        };
        match next {
            Ok(accumulation) => {
                *self.accumulation.borrow_mut() = Some(accumulation.clone());
                self.sink.forward_on_next(accumulation);
            }
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

pub(crate) fn scan_seed<T: 'static, A: Clone + 'static>(source: Rc<dyn IObservable<T>>, seed: A, accumulator: Accumulator<A, T>) -> Rc<dyn IObservable<A>> {
    unary(source, move |observer| {
        Rc::new(ScanSeedSink { sink: Sink::new(observer), accumulator: accumulator.clone(), accumulation: RefCell::new(Some(seed.clone())) })
    })
}

pub(crate) fn scan<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, accumulator: Accumulator<T, T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(ScanSink { sink: Sink::new(observer), accumulator: accumulator.clone(), accumulation: RefCell::new(None) }))
}

/// The methods of `Scan` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! scan_methods {
    () => {
        /// Each accumulation of the values, the first value being the first accumulation
        /// (`Scan(Func<TSource, TSource, TSource>)`).
        fn scan(&self, accumulator: impl Fn(T, T) -> T + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::scan::scan(self.observable(), std::rc::Rc::new(move |accumulation, value| Ok(accumulator(accumulation, value))))
        }

        /// Each accumulation of the values from `seed`
        /// (`Scan(TAccumulate, Func<TAccumulate, TSource, TAccumulate>)`).
        fn scan_with<A: Clone + 'static>(
            &self,
            seed: A,
            accumulator: impl Fn(A, T) -> A + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> {
            crate::linq::scan::scan_seed(self.observable(), seed, std::rc::Rc::new(move |accumulation, value| Ok(accumulator(accumulation, value))))
        }

        /// [`scan`](Self::scan) with an accumulator that may fail; a failure ends the sequence
        /// with its error (`Scan(Func<TSource, TSource, TSource>)` with an accumulator that throws).
        fn try_scan(
            &self,
            accumulator: impl Fn(T, T) -> Result<T, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::scan::scan(self.observable(), std::rc::Rc::new(accumulator))
        }

        /// [`scan_with`](Self::scan_with) with an accumulator that may fail; a failure ends the
        /// sequence with its error (`Scan(TAccumulate, Func<TAccumulate, TSource, TAccumulate>)`
        /// with an accumulator that throws).
        fn try_scan_with<A: Clone + 'static>(
            &self,
            seed: A,
            accumulator: impl Fn(A, T) -> Result<A, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<A>> {
            crate::linq::scan::scan_seed(self.observable(), seed, std::rc::Rc::new(accumulator))
        }
    };
}
pub(crate) use scan_methods;
