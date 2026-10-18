//! `LastAsync` (`Linq/Observable/LastAsync.cs`) and `LastOrDefaultAsync`
//! (`Linq/Observable/LastOrDefaultAsync.cs`): the last value as a sequence of one value.

use super::aggregate::SequenceError;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

type Predicate<T> = Rc<dyn Fn(&T) -> Result<bool, ObservableError>>;

/// Whether `predicate` (none: every value) is true for `value`.
fn matches<T>(predicate: &Option<Predicate<T>>, value: &T) -> Result<bool, ObservableError> {
    match predicate {
        None => Ok(true),
        Some(predicate) => predicate(value),
    }
}

/// The sink of `LastAsync` and `LastOrDefaultAsync` (`LastAsync<TSource>.Sequence._`,
/// `LastAsync<TSource>.Predicate._`, `LastOrDefaultAsync<TSource>.Sequence._` and
/// `LastOrDefaultAsync<TSource>.Predicate._`): keeps the last value (the predicate is true for) and
/// gives it at the end. A sequence without one gives the default value when `default` is set, and
/// else ends with [`SequenceError::NoElements`] (or [`SequenceError::NoMatchingElements`]).
struct LastSink<T> {
    sink: Sink<T>,
    predicate: Option<Predicate<T>>,
    default: Option<fn() -> T>,
    value: RefCell<Option<T>>,
}

impl<T: 'static> OperatorSink<T, T> for LastSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(LastSink<T>);

impl<T: 'static> IObserver<T> for LastSink<T> {
    fn on_next(&self, value: T) {
        match matches(&self.predicate, &value) {
            Ok(true) => drop(self.value.replace(Some(value))),
            Ok(false) => {}
            Err(error) => {
                drop(self.value.take());
                self.sink.forward_on_error(error);
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        drop(self.value.take());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let value = match (self.value.take(), self.default) {
            (Some(value), _) => value,
            (None, Some(default)) => default(),
            (None, None) => {
                let error = if self.predicate.is_none() { SequenceError::NoElements } else { SequenceError::NoMatchingElements };
                self.sink.forward_on_error(error.into_error());
                return;
            }
        };
        self.sink.forward_on_next(value);
        self.sink.forward_on_completed();
    }
}

fn last_sinks<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Option<Predicate<T>>, default: Option<fn() -> T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(LastSink { sink: Sink::new(observer), predicate: predicate.clone(), default, value: RefCell::new(None) }))
}

pub(crate) fn last_async<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    last_sinks(source, None, None)
}

pub(crate) fn last_async_with<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    last_sinks(source, Some(predicate), None)
}

pub(crate) fn last_or_default_async<T: Default + 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    last_sinks(source, None, Some(T::default as fn() -> T))
}

pub(crate) fn last_or_default_async_with<T: Default + 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    last_sinks(source, Some(predicate), Some(T::default as fn() -> T))
}

/// The methods of `LastAsync` and `LastOrDefaultAsync` (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! last_async_methods {
    () => {
        /// The last value, as a sequence of one value; a sequence without values ends with
        /// `SequenceError::NoElements` (`LastAsync()`).
        fn last_async(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::last_async::last_async(self.observable())
        }

        /// The last value `predicate` is true for; a sequence without one ends with
        /// `SequenceError::NoMatchingElements` (`LastAsync(Func<TSource, bool>)`).
        fn last_async_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::last_async::last_async_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`last_async_with`](Self::last_async_with) with a predicate that may fail; a failure
        /// ends the sequence with its error (`LastAsync(Func<TSource, bool>)` with a predicate
        /// that throws).
        fn try_last_async_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::last_async::last_async_with(self.observable(), std::rc::Rc::new(predicate))
        }

        /// The last value, or the default value of `T` when the sequence has none
        /// (`LastOrDefaultAsync()`).
        fn last_or_default_async(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::last_async::last_or_default_async(self.observable())
        }

        /// The last value `predicate` is true for, or the default value of `T`
        /// (`LastOrDefaultAsync(Func<TSource, bool>)`).
        fn last_or_default_async_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::last_async::last_or_default_async_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`last_or_default_async_with`](Self::last_or_default_async_with) with a predicate that
        /// may fail; a failure ends the sequence with its error
        /// (`LastOrDefaultAsync(Func<TSource, bool>)` with a predicate that throws).
        fn try_last_or_default_async_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::last_async::last_or_default_async_with(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use last_async_methods;
