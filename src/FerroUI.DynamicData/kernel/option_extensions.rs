//! `OptionExtensions` (`Kernel/OptionExtensions.cs`): the extensions of `Optional<T>`, which is
//! `Option<T>` in the port, and the lookups that return one.
//!
//! The overloads that the standard library already has keep its form: `OrElse` is
//! `Option::or_else`, `ValueOr(T?, T)` of a nullable value is `Option::unwrap_or`, and
//! `IfHasValue(Optional<T>?)` of a nullable option is `Option::flatten` followed by
//! [`if_has_value`](OptionExtensions::if_has_value).

use super::OptionElse;
use std::collections::HashMap;
use std::hash::Hash;

/// The extensions of an option (`OptionExtensions` on `Optional<T>`).
pub trait OptionExtensions<T>: Sized {
    /// The value converted by `converter`, or none (`Convert(Func<TSource, TDestination>)`).
    fn convert<U>(self, converter: impl FnOnce(T) -> U) -> Option<U>;

    /// The option that `converter` makes of the value, or none
    /// (`Convert(Func<TSource, Optional<TDestination>>)`).
    fn convert_optional<U>(self, converter: impl FnOnce(T) -> Option<U>) -> Option<U>;

    /// The value converted by `converter`, or what `fallback_converter` makes when there is none
    /// (`ConvertOr`).
    fn convert_or<U>(self, converter: impl FnOnce(T) -> U, fallback_converter: impl FnOnce() -> U) -> U;

    /// Runs `action` with the value; the result runs its `else_` action when there is none
    /// (`IfHasValue(Action<T>)`).
    fn if_has_value(self, action: impl FnOnce(T)) -> OptionElse;

    /// The value, or what `value_selector` makes when there is none (`ValueOr(Func<T>)`).
    fn value_or(self, value_selector: impl FnOnce() -> T) -> T;

    /// The value, or the default value of `T` (`ValueOrDefault`).
    fn value_or_default(self) -> T
    where
        T: Default;

    /// The value, or the error that `exception_generator` makes when there is none
    /// (`ValueOrThrow`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): upstream throws the exception; the port returns it
    /// as the `Err` of the result.
    fn value_or_throw<E>(self, exception_generator: impl FnOnce() -> E) -> Result<T, E>;
}

impl<T> OptionExtensions<T> for Option<T> {
    fn convert<U>(self, converter: impl FnOnce(T) -> U) -> Option<U> {
        self.map(converter)
    }

    fn convert_optional<U>(self, converter: impl FnOnce(T) -> Option<U>) -> Option<U> {
        self.and_then(converter)
    }

    fn convert_or<U>(self, converter: impl FnOnce(T) -> U, fallback_converter: impl FnOnce() -> U) -> U {
        match self {
            Some(value) => converter(value),
            None => fallback_converter(),
        }
    }

    fn if_has_value(self, action: impl FnOnce(T)) -> OptionElse {
        match self {
            Some(value) => {
                action(value);
                OptionElse::no_action()
            }
            None => OptionElse::run(),
        }
    }

    fn value_or(self, value_selector: impl FnOnce() -> T) -> T {
        self.unwrap_or_else(value_selector)
    }

    fn value_or_default(self) -> T
    where
        T: Default,
    {
        self.unwrap_or_default()
    }

    fn value_or_throw<E>(self, exception_generator: impl FnOnce() -> E) -> Result<T, E> {
        self.ok_or_else(exception_generator)
    }
}

/// The first item that `selector` accepts, or none (`FirstOrOptional`).
pub fn first_or_optional<T>(source: impl IntoIterator<Item = T>, selector: impl Fn(&T) -> bool) -> Option<T> {
    source.into_iter().find(|item| selector(item))
}

/// The value of `key` in `source`, or none (`Lookup(IDictionary, TKey)`).
pub fn lookup<K: Eq + Hash, V: Clone>(source: &HashMap<K, V>, key: &K) -> Option<V> {
    source.get(key).cloned()
}

/// Removes `key` from `source`; whether it was there (`RemoveIfContained`).
pub fn remove_if_contained<K: Eq + Hash, V>(source: &mut HashMap<K, V>, key: &K) -> bool {
    source.remove(key).is_some()
}

/// The values of the options that have one (`SelectValues(IEnumerable<Optional<T>>)`).
pub fn select_values<T>(source: impl IntoIterator<Item = Option<T>>) -> Vec<T> {
    source.into_iter().flatten().collect()
}
