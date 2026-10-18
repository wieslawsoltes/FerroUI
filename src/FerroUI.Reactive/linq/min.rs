//! `Min` (`Linq/Observable/Min.cs`), and the sink `Max` shares with it.

use super::aggregate::SequenceError;
use super::numeric::Extremum;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

/// Whether a value replaces the current extremum; an error ends the sequence with it (a comparer
/// that throws).
pub(crate) type Replaces<T> = Rc<dyn Fn(&T, &T) -> Result<bool, ObservableError>>;

/// A comparer of values (`IComparer<T>`) that may fail.
pub(crate) type Comparer<T> = Rc<dyn Fn(&T, &T) -> Result<Ordering, ObservableError>>;

/// The sink of `Min` and `Max` (`Min<TSource>.NonNull` and `Min<TSource>.Null`, `MinInt32._`,
/// `MinDouble._`, `MinDoubleNullable._`, ..., and the same of `Max`): keeps the value the others
/// do not replace and gives it at the end. Values that are null are ignored; a sequence without a
/// value gives the empty value of the type, or ends with [`SequenceError::NoElements`] when it has
/// none.
pub(crate) struct ExtremumSink<T> {
    sink: Sink<T>,
    replaces: Replaces<T>,
    is_null: fn(&T) -> bool,
    empty: fn() -> Option<T>,
    last: RefCell<Option<T>>,
}

impl<T: 'static> OperatorSink<T, T> for ExtremumSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(ExtremumSink<T>);

impl<T: 'static> IObserver<T> for ExtremumSink<T> {
    fn on_next(&self, value: T) {
        if (self.is_null)(&value) {
            return;
        }
        let replaces = match self.last.borrow().as_ref() {
            None => Ok(true),
            Some(current) => (self.replaces)(&value, current),
        };
        match replaces {
            Ok(true) => *self.last.borrow_mut() = Some(value),
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        match self.last.take().or_else(self.empty) {
            Some(value) => {
                self.sink.forward_on_next(value);
                self.sink.forward_on_completed();
            }
            None => self.sink.forward_on_error(SequenceError::NoElements.into_error()),
        }
    }
}

/// The observable of `Min` or `Max`: `replaces` decides whether a value replaces the current one,
/// `is_null` which values are ignored, `empty` the value of a sequence without a value.
pub(crate) fn extremum<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    replaces: Replaces<T>,
    is_null: fn(&T) -> bool,
    empty: fn() -> Option<T>,
) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(ExtremumSink { sink: Sink::new(observer), replaces: replaces.clone(), is_null, empty, last: RefCell::new(None) })
    })
}

/// No value of a comparer form is null (`Min<TSource>.NonNull`).
pub(crate) fn never_null<T>(_value: &T) -> bool {
    false
}

/// A sequence without a value of a comparer form ends with [`SequenceError::NoElements`].
pub(crate) fn no_empty_value<T>() -> Option<T> {
    None
}

pub(crate) fn min<T: Extremum>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    extremum(source, Rc::new(|value: &T, current: &T| Ok(value.replaces_min(current))), T::is_null, T::empty)
}

pub(crate) fn min_with<T: 'static>(source: Rc<dyn IObservable<T>>, comparer: Comparer<T>) -> Rc<dyn IObservable<T>> {
    // Deviation (DEVIATIONS.md, Reactive): the comparer forms have no null: upstream's
    // `Min<TSource>(IComparer<TSource>)` ignores the nulls of a reference or nullable type and gives
    // null for a sequence without a value; here every value is compared (a `None` of an `Option`
    // sequence too) and a sequence without a value ends with `SequenceError::NoElements`.
    extremum(
        source,
        Rc::new(move |value: &T, current: &T| comparer(value, current).map(|comparison| comparison == Ordering::Less)),
        never_null::<T>,
        no_empty_value::<T>,
    )
}

/// The methods of `Min` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! min_methods {
    () => {
        /// The minimum value (`Min()` of `int`, `long`, `float`, `double` and their nullable forms,
        /// and `Min<TSource>()` with the default comparer): a sequence without a value ends with
        /// `SequenceError::NoElements`; an `Option` sequence ignores its `None` values and gives
        /// `None` without a `Some`; a NaN `f32` or `f64` is the minimum.
        fn min(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: crate::linq::numeric::Extremum,
        {
            crate::linq::min::min(self.observable())
        }

        /// The minimum value by `comparer`; a sequence without a value ends with
        /// `SequenceError::NoElements` (`Min(IComparer<TSource>)`).
        fn min_with(&self, comparer: impl Fn(&T, &T) -> std::cmp::Ordering + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::min::min_with(self.observable(), std::rc::Rc::new(move |x: &T, y: &T| Ok(comparer(x, y))))
        }

        /// [`min_with`](Self::min_with) with a comparer that may fail; a failure ends the sequence
        /// with its error (`Min(IComparer<TSource>)` with a comparer that throws).
        fn try_min_with(
            &self,
            comparer: impl Fn(&T, &T) -> Result<std::cmp::Ordering, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::min::min_with(self.observable(), std::rc::Rc::new(comparer))
        }

        /// The minimum of the values projected with `selector` (`Min(Func<TSource, TResult>)` and
        /// the overloads for `int`, `long`, `float`, `double` and their nullable forms).
        fn min_of<R: crate::linq::numeric::Extremum>(&self, selector: impl Fn(T) -> R + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::min::min(crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value)))))
        }

        /// The minimum by `comparer` of the values projected with `selector`
        /// (`Min(Func<TSource, TResult>, IComparer<TResult>)`).
        fn min_of_with<R: 'static>(
            &self,
            selector: impl Fn(T) -> R + 'static,
            comparer: impl Fn(&R, &R) -> std::cmp::Ordering + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::min::min_with(
                crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value)))),
                std::rc::Rc::new(move |x: &R, y: &R| Ok(comparer(x, y))),
            )
        }
    };
}
pub(crate) use min_methods;
