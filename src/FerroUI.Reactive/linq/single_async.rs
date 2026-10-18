//! `SingleAsync` (`Linq/Observable/SingleAsync.cs`) and `SingleOrDefaultAsync`
//! (`Linq/Observable/SingleOrDefaultAsync.cs`): the only value as a sequence of one value.

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

/// The sink of `SingleAsync` and `SingleOrDefaultAsync` (`SingleAsync<TSource>.Sequence._`,
/// `SingleAsync<TSource>.Predicate._`, `SingleOrDefaultAsync<TSource>.Sequence._` and
/// `SingleOrDefaultAsync<TSource>.Predicate._`): a second value (the predicate is true for) ends the
/// sequence with [`SequenceError::MoreThanOneElement`] (or
/// [`SequenceError::MoreThanOneMatchingElement`]); the end gives the value. A sequence without one
/// gives the default value when `default` is set, and else ends with [`SequenceError::NoElements`]
/// (or [`SequenceError::NoMatchingElements`]).
struct SingleSink<T> {
    sink: Sink<T>,
    predicate: Option<Predicate<T>>,
    default: Option<fn() -> T>,
    value: RefCell<Option<T>>,
}

impl<T: 'static> OperatorSink<T, T> for SingleSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(SingleSink<T>);

impl<T: 'static> IObserver<T> for SingleSink<T> {
    fn on_next(&self, value: T) {
        match matches(&self.predicate, &value) {
            Ok(true) => {
                if self.value.borrow().is_some() {
                    let error = if self.predicate.is_none() { SequenceError::MoreThanOneElement } else { SequenceError::MoreThanOneMatchingElement };
                    self.sink.forward_on_error(error.into_error());
                    return;
                }
                *self.value.borrow_mut() = Some(value);
            }
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
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

fn single_sinks<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Option<Predicate<T>>, default: Option<fn() -> T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(SingleSink { sink: Sink::new(observer), predicate: predicate.clone(), default, value: RefCell::new(None) }))
}

pub(crate) fn single_async<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    single_sinks(source, None, None)
}

pub(crate) fn single_async_with<T: 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    single_sinks(source, Some(predicate), None)
}

pub(crate) fn single_or_default_async<T: Default + 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    single_sinks(source, None, Some(T::default as fn() -> T))
}

pub(crate) fn single_or_default_async_with<T: Default + 'static>(source: Rc<dyn IObservable<T>>, predicate: Predicate<T>) -> Rc<dyn IObservable<T>> {
    single_sinks(source, Some(predicate), Some(T::default as fn() -> T))
}

/// The methods of `SingleAsync` and `SingleOrDefaultAsync` (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! single_async_methods {
    () => {
        /// The only value, as a sequence of one value; a sequence without values ends with
        /// `SequenceError::NoElements`, one with more than one with
        /// `SequenceError::MoreThanOneElement` (`SingleAsync()`).
        fn single_async(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::single_async::single_async(self.observable())
        }

        /// The only value `predicate` is true for; a sequence without one ends with
        /// `SequenceError::NoMatchingElements`, one with more than one with
        /// `SequenceError::MoreThanOneMatchingElement` (`SingleAsync(Func<TSource, bool>)`).
        fn single_async_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::single_async::single_async_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`single_async_with`](Self::single_async_with) with a predicate that may fail; a
        /// failure ends the sequence with its error (`SingleAsync(Func<TSource, bool>)` with a
        /// predicate that throws).
        fn try_single_async_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::single_async::single_async_with(self.observable(), std::rc::Rc::new(predicate))
        }

        /// The only value, or the default value of `T` when the sequence has none; one with more
        /// than one ends with `SequenceError::MoreThanOneElement` (`SingleOrDefaultAsync()`).
        fn single_or_default_async(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::single_async::single_or_default_async(self.observable())
        }

        /// The only value `predicate` is true for, or the default value of `T`
        /// (`SingleOrDefaultAsync(Func<TSource, bool>)`).
        fn single_or_default_async_with(&self, predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::single_async::single_or_default_async_with(self.observable(), std::rc::Rc::new(move |value: &T| Ok(predicate(value))))
        }

        /// [`single_or_default_async_with`](Self::single_or_default_async_with) with a predicate
        /// that may fail; a failure ends the sequence with its error
        /// (`SingleOrDefaultAsync(Func<TSource, bool>)` with a predicate that throws).
        fn try_single_or_default_async_with(
            &self,
            predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Default,
        {
            crate::linq::single_async::single_or_default_async_with(self.observable(), std::rc::Rc::new(predicate))
        }
    };
}
pub(crate) use single_async_methods;
