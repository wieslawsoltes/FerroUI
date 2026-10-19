//! `OptionObservableExtensions` (`Kernel/OptionObservableExtensions.cs`): the operators of
//! sequences of options (`IObservable<Optional<T>>`).

use super::OptionExtensions;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The operators of sequences of options (`OptionObservableExtensions`), implemented for
/// `Rc<dyn IObservable<Option<T>>>`.
///
/// It also carries `EditDiff` of a sequence of options (`ObservableCacheEx.EditDiff(
/// IObservable<Optional<TObject>>, ..)`), as [`edit_diff_optional`](Self::edit_diff_optional).
pub trait OptionObservableExtensions<T: Clone + 'static> {
    /// The sequence as the handle the operators take.
    fn options(&self) -> Rc<dyn IObservable<Option<T>>>;

    /// Each value converted by `converter`; none stays none (`Convert(Func<TSource, TDestination>)`).
    fn convert<U: 'static>(&self, converter: impl Fn(T) -> U + 'static) -> Rc<dyn IObservable<Option<U>>> {
        self.options().select(move |optional: Option<T>| optional.map(&converter))
    }

    /// The option that `converter` makes of each value; none stays none
    /// (`Convert(Func<TSource, Optional<TDestination>>)`).
    fn convert_optional<U: 'static>(&self, converter: impl Fn(T) -> Option<U> + 'static) -> Rc<dyn IObservable<Option<U>>> {
        self.options().select(move |optional: Option<T>| optional.and_then(&converter))
    }

    /// Each value converted by `converter`, and what `fallback_converter` makes for none
    /// (`ConvertOr`).
    fn convert_or<U: 'static>(&self, converter: impl Fn(T) -> U + 'static, fallback_converter: impl Fn() -> U + 'static) -> Rc<dyn IObservable<U>> {
        self.options().select(move |optional: Option<T>| match optional {
            Some(value) => converter(value),
            None => fallback_converter(),
        })
    }

    /// The options, with what `fallback_operation` makes in place of none (`OrElse`).
    fn or_else(&self, fallback_operation: impl Fn() -> Option<T> + 'static) -> Rc<dyn IObservable<Option<T>>> {
        self.options().select(move |optional: Option<T>| match optional {
            Some(value) => Some(value),
            None => fallback_operation(),
        })
    }

    /// Runs `action` with each value (`OnHasValue(Action<T>)`).
    fn on_has_value(&self, action: impl Fn(&T) + 'static) -> Rc<dyn IObservable<Option<T>>> {
        self.options().do_(move |optional: &Option<T>| {
            optional.as_ref().if_has_value(|value| action(value));
        })
    }

    /// Runs `action` with each value, and `else_action` for each none
    /// (`OnHasValue(Action<T>, Action)`).
    fn on_has_value_with_else(&self, action: impl Fn(&T) + 'static, else_action: impl Fn() + 'static) -> Rc<dyn IObservable<Option<T>>> {
        self.options().do_(move |optional: &Option<T>| {
            optional.as_ref().if_has_value(|value| action(value)).else_(&else_action);
        })
    }

    /// Runs `action` for each none (`OnHasNoValue(Action)`).
    fn on_has_no_value(&self, action: impl Fn() + 'static) -> Rc<dyn IObservable<Option<T>>> {
        self.options().do_(move |optional: &Option<T>| {
            optional.as_ref().if_has_value(|_| {}).else_(&action);
        })
    }

    /// Runs `action` for each none, and `else_action` with each value
    /// (`OnHasNoValue(Action, Action<T>)`).
    fn on_has_no_value_with_else(&self, action: impl Fn() + 'static, else_action: impl Fn(&T) + 'static) -> Rc<dyn IObservable<Option<T>>> {
        self.options().do_(move |optional: &Option<T>| {
            optional.as_ref().if_has_value(|value| else_action(value)).else_(&action);
        })
    }

    /// The values; the nones are left out (`SelectValues`).
    fn select_values(&self) -> Rc<dyn IObservable<T>> {
        self.options().where_(|optional: &Option<T>| optional.is_some()).select(|optional: Option<T>| optional.expect("filtered to values"))
    }

    /// The values, with what `value_selector` makes in place of none (`ValueOr`).
    fn value_or(&self, value_selector: impl Fn() -> T + 'static) -> Rc<dyn IObservable<T>> {
        self.options().select(move |optional: Option<T>| optional.unwrap_or_else(&value_selector))
    }

    /// The values, with the default value of `T` in place of none (`ValueOrDefault`).
    fn value_or_default(&self) -> Rc<dyn IObservable<T>>
    where
        T: Default,
    {
        self.options().select(|optional: Option<T>| optional.unwrap_or_default())
    }

    /// The values; a none ends the sequence with the error of `exception_generator`
    /// (`ValueOrThrow`).
    fn value_or_throw(&self, exception_generator: impl Fn() -> ObservableError + 'static) -> Rc<dyn IObservable<T>> {
        let source = self.options();
        let exception_generator = Rc::new(exception_generator);
        Observable::create(move |observer: Rc<dyn IObserver<T>>| {
            let (on_next, on_error, on_completed) = (observer.clone(), observer.clone(), observer);
            let exception_generator = exception_generator.clone();
            source.subscribe_all(
                move |optional: Option<T>| match optional {
                    Some(value) => on_next.on_next(value),
                    None => on_next.on_error(exception_generator()),
                },
                move |error| on_error.on_error(error),
                move || on_completed.on_completed(),
            )
        })
    }

    /// The changes of a cache that holds the value of the option, keyed by `key_selector`: a
    /// value adds it, another value with the same key updates it unless it is equal, one with
    /// another key replaces it, and none removes it (`EditDiff(IObservable<Optional<TObject>>,
    /// Func<TObject, TKey>)`, equality of `PartialEq`).
    fn edit_diff_optional<K: Eq + std::hash::Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        crate::cache::internal::edit_diff_change_set_optional(self.options(), Rc::new(key_selector), Rc::new(|a: &T, b: &T| a == b))
    }

    /// [`edit_diff_optional`](Self::edit_diff_optional) with the equality of `equality_comparer`
    /// (`EditDiff(.., IEqualityComparer<TObject>)`).
    fn edit_diff_optional_with_comparer<K: Eq + std::hash::Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
        equality_comparer: impl Fn(&T, &T) -> bool + 'static,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        crate::cache::internal::edit_diff_change_set_optional(self.options(), Rc::new(key_selector), Rc::new(equality_comparer))
    }
}

impl<T: Clone + 'static> OptionObservableExtensions<T> for Rc<dyn IObservable<Option<T>>> {
    fn options(&self) -> Rc<dyn IObservable<Option<T>>> {
        self.clone()
    }
}
