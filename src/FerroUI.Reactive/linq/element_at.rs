//! `ElementAt` (`Linq/Observable/ElementAt.cs`) and `ElementAtOrDefault`
//! (`Linq/Observable/ElementAtOrDefault.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// The error of an argument outside of the range of its valid values
/// (`ArgumentOutOfRangeException`): the error of `ElementAt` when the sequence ends before the
/// element at the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentOutOfRangeError {
    param_name: &'static str,
}

impl ArgumentOutOfRangeError {
    /// The error of the argument `param_name`.
    pub fn new(param_name: &'static str) -> Self {
        Self { param_name }
    }

    /// The name of the argument (`ParamName`).
    pub fn param_name(&self) -> &'static str {
        self.param_name
    }
}

impl fmt::Display for ArgumentOutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Specified argument was out of the range of valid values. (Parameter '{}')", self.param_name)
    }
}

impl std::error::Error for ArgumentOutOfRangeError {}

/// The sink of `ElementAt` (`ElementAt<TSource>._`). `remaining` counts down the values before the
/// element; it is `None` once the element went by (upstream's counter below zero).
struct ElementAtSink<T> {
    sink: Sink<T>,
    remaining: Cell<Option<usize>>,
}

impl<T: 'static> OperatorSink<T, T> for ElementAtSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(ElementAtSink<T>);

impl<T: 'static> IObserver<T> for ElementAtSink<T> {
    fn on_next(&self, value: T) {
        let remaining = self.remaining.get();
        if remaining == Some(0) {
            self.sink.forward_on_next(value);
            self.sink.forward_on_completed();
        }
        self.remaining.set(remaining.and_then(|n| n.checked_sub(1)));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        if self.remaining.get().is_some() {
            self.sink.forward_on_error(Rc::new(ArgumentOutOfRangeError::new("index")));
        }
    }
}

pub(crate) fn element_at<T: 'static>(source: Rc<dyn IObservable<T>>, index: usize) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(ElementAtSink { sink: Sink::new(observer), remaining: Cell::new(Some(index)) }))
}

/// The sink of `ElementAtOrDefault` (`ElementAtOrDefault<TSource>._`).
struct ElementAtOrDefaultSink<T> {
    sink: Sink<T>,
    remaining: Cell<Option<usize>>,
}

impl<T: Default + 'static> OperatorSink<T, T> for ElementAtOrDefaultSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(ElementAtOrDefaultSink<T> where T: Default);

impl<T: Default + 'static> IObserver<T> for ElementAtOrDefaultSink<T> {
    fn on_next(&self, value: T) {
        let remaining = self.remaining.get();
        if remaining == Some(0) {
            self.sink.forward_on_next(value);
            self.sink.forward_on_completed();
        }
        self.remaining.set(remaining.and_then(|n| n.checked_sub(1)));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_next(T::default());
        self.sink.forward_on_completed();
    }
}

pub(crate) fn element_at_or_default<T: Default + 'static>(source: Rc<dyn IObservable<T>>, index: usize) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(ElementAtOrDefaultSink { sink: Sink::new(observer), remaining: Cell::new(Some(index)) }))
}

/// The methods of `ElementAt` and `ElementAtOrDefault` (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! element_at_methods {
    () => {
        /// The value at `index`, then the end; the sequence ends with an `ArgumentOutOfRangeError`
        /// when the source ends before (`ElementAt(int)`).
        fn element_at(&self, index: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::element_at::element_at(self.observable(), index)
        }

        /// The value at `index`, or the default value when the source ends before, then the end
        /// (`ElementAtOrDefault(int)`).
        fn element_at_or_default(&self, index: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::element_at::element_at_or_default(self.observable(), index)
        }
    };
}
pub(crate) use element_at_methods;
