//! `FirstAsync` (`Linq/Observable/FirstAsync.cs`) and `FirstOrDefaultAsync`
//! (`Linq/Observable/FirstOrDefaultAsync.cs`): the first value as a sequence of one value.

use super::aggregate::SequenceError;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T) -> Result<bool, ObservableError>>;

/// Whether `predicate` (none: every value) is true for `value`.
fn matches<T>(predicate: &Option<Predicate<T>>, value: &T) -> Result<bool, ObservableError> {
    match predicate {
        None => Ok(true),
        Some(predicate) => predicate(value),
    }
}

/// The sink of `FirstAsync` (`FirstAsync<TSource>.Sequence._` without a predicate,
/// `FirstAsync<TSource>.Predicate._` with one): the first value (the predicate is true for) ends
/// the sequence; a sequence without one ends with [`SequenceError::NoElements`] (or
/// [`SequenceError::NoMatchingElements`]).
struct FirstSink<T> {
    sink: Sink<T>,
    predicate: Option<Predicate<T>>,
    found: Cell<bool>,
}

impl<T: 'static> OperatorSink<T, T> for FirstSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(FirstSink<T>);

impl<T: 'static> IObserver<T> for FirstSink<T> {
    fn on_next(&self, value: T) {
        match matches(&self.predicate, &value) {
            Ok(true) => {
                self.found.set(true);
                self.sink.forward_on_next(value);
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
        if !self.found.get() {
            let error = if self.predicate.is_none() { SequenceError::NoElements } else { SequenceError::NoMatchingElements };
            self.sink.forward_on_error(error.into_error());
        }
    }
}

/// The sink of `FirstOrDefaultAsync` (`FirstOrDefaultAsync<TSource>.Sequence._` and
/// `FirstOrDefaultAsync<TSource>.Predicate._`): the first value (the predicate is true for) ends
/// the sequence; a sequence without one gives the default value.
struct FirstOrDefaultSink<T> {
    sink: Sink<T>,
    predicate: Option<Predicate<T>>,
}

impl<T: Default + 'static> OperatorSink<T, T> for FirstOrDefaultSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(FirstOrDefaultSink<T> where T: Default);

impl<T: Default + 'static> IObserver<T> for FirstOrDefaultSink<T> {
    fn on_next(&self, value: T) {
        match matches(&self.predicate, &value) {
            Ok(true) => {
                self.sink.forward_on_next(value);
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
        self.sink.forward_on_next(T::default());
        self.sink.forward_on_completed();
    }
}

fn first_sinks<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Option<Predicate<T>>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(FirstSink { sink: Sink::new(observer), predicate: predicate.clone(), found: Cell::new(false) }))
}

fn first_or_default_sinks<T: Default + 'static>(source: Rc<dyn IObservable<T>>, predicate: Option<Predicate<T>>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(FirstOrDefaultSink { sink: Sink::new(observer), predicate: predicate.clone() }))
}

pub(crate) fn first_async<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    first_sinks(source, None)
}

pub(crate) fn first_async_with<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    first_sinks(source, Some(predicate))
}

pub(crate) fn first_or_default_async<T: Default + 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    first_or_default_sinks(source, None)
}

pub(crate) fn first_or_default_async_with<T: Default + 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    first_or_default_sinks(source, Some(predicate))
}

/// The methods of `FirstAsync` and `FirstOrDefaultAsync` (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! first_async_methods {
    () => {
        /// The first value, as a sequence of one value; a sequence without values ends with
        /// `SequenceError::NoElements` (`FirstAsync()`).
        fn first_async(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::first_async::first_async(self.observable())
        }

        /// The first value `predicate` is true for; a sequence without one ends with
        /// `SequenceError::NoMatchingElements` (`FirstAsync(Func<TSource, bool>)`).
        fn first_async_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::first_async::first_async_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`first_async_with`](Self::first_async_with) with a predicate that may fail; a failure
        /// ends the sequence with its error (`FirstAsync(Func<TSource, bool>)` with a predicate
        /// that throws).
        fn try_first_async_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::first_async::first_async_with(self.observable(), std::rc::Rc::new(predicate))
        }

        /// The first value, or the default value of `T` when the sequence has none
        /// (`FirstOrDefaultAsync()`).
        fn first_or_default_async(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::first_async::first_or_default_async(self.observable())
        }

        /// The first value `predicate` is true for, or the default value of `T`
        /// (`FirstOrDefaultAsync(Func<TSource, bool>)`).
        fn first_or_default_async_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::first_async::first_or_default_async_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`first_or_default_async_with`](Self::first_or_default_async_with) with a predicate
        /// that may fail; a failure ends the sequence with its error
        /// (`FirstOrDefaultAsync(Func<TSource, bool>)` with a predicate that throws).
        fn try_first_or_default_async_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::first_async::first_or_default_async_with(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use first_async_methods;
