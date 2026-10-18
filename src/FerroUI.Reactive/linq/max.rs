//! `Max` (`Linq/Observable/Max.cs`); its sink is the one of `Min` (`super::min::ExtremumSink`).

use super::min::{extremum, never_null, no_empty_value, Comparer};
use super::numeric::Extremum;
use ferroui_system::IObservable;
use std::cmp::Ordering;
use std::rc::Rc;

pub(crate) fn max<T: Extremum>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    extremum(source, Rc::new(|value: &T, current: &T| Ok(value.replaces_max(current))), T::is_null, T::empty)
}

pub(crate) fn max_with<T: 'static>(source: Rc<dyn IObservable<T>>, comparer: Comparer<T>) -> Rc<dyn IObservable<T>> {
    // Deviation (DEVIATIONS.md, Reactive): the comparer forms have no null, as `min_with`.
    extremum(
        source,
        Rc::new(move |value: &T, current: &T| comparer(value, current).map(|comparison| comparison == Ordering::Greater)),
        never_null::<T>,
        no_empty_value::<T>,
    )
}

/// The methods of `Max` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! max_methods {
    () => {
        /// The maximum value (`Max()` of `int`, `long`, `float`, `double` and their nullable forms,
        /// and `Max<TSource>()` with the default comparer): a sequence without a value ends with
        /// `SequenceError::NoElements`; an `Option` sequence ignores its `None` values and gives
        /// `None` without a `Some`; a NaN `f32` or `f64` is the maximum.
        fn max(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: crate::linq::numeric::Extremum,
        {
            crate::linq::max::max(self.observable())
        }

        /// The maximum value by `comparer`; a sequence without a value ends with
        /// `SequenceError::NoElements` (`Max(IComparer<TSource>)`).
        fn max_with(&self, comparer: impl Fn(&T, &T) -> std::cmp::Ordering + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::max::max_with(self.observable(), std::rc::Rc::new(move |x: &T, y: &T| Ok(comparer(x, y))))
        }

        /// [`max_with`](Self::max_with) with a comparer that may fail; a failure ends the sequence
        /// with its error (`Max(IComparer<TSource>)` with a comparer that throws).
        fn try_max_with(
            &self,
            comparer: impl Fn(&T, &T) -> Result<std::cmp::Ordering, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::max::max_with(self.observable(), std::rc::Rc::new(comparer))
        }

        /// The maximum of the values projected with `selector` (`Max(Func<TSource, TResult>)` and
        /// the overloads for `int`, `long`, `float`, `double` and their nullable forms).
        fn max_of<R: crate::linq::numeric::Extremum>(&self, selector: impl Fn(T) -> R + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::max::max(crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value)))))
        }

        /// The maximum by `comparer` of the values projected with `selector`
        /// (`Max(Func<TSource, TResult>, IComparer<TResult>)`).
        fn max_of_with<R: 'static>(
            &self,
            selector: impl Fn(T) -> R + 'static,
            comparer: impl Fn(&R, &R) -> std::cmp::Ordering + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::max::max_with(
                crate::linq::select::select(self.observable(), std::rc::Rc::new(move |value, _| Ok(selector(value)))),
                std::rc::Rc::new(move |x: &R, y: &R| Ok(comparer(x, y))),
            )
        }
    };
}
pub(crate) use max_methods;
