//! `IComparerBuilder<T>` (`ReactiveUI.Core/Comparers/IComparerBuilder.cs`).

use super::Comparer;

/// The starting point of a chain of comparers (`IComparerBuilder<T>`): the first key of the
/// order, which `then_by` extends ([`ComparerChainingExtensions`](super::ComparerChainingExtensions)).
pub trait IComparerBuilder<T> {
    /// In ascending order of the values of `selector`, compared by their `Ord`
    /// (`OrderBy<TValue>(Func<T, TValue>)`).
    fn order_by<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T>;

    /// In ascending order of the values of `selector`, compared by `comparer`
    /// (`OrderBy<TValue>(Func<T, TValue>, IComparer<TValue>)`).
    fn order_by_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T>;

    /// In descending order of the values of `selector`, compared by their `Ord`
    /// (`OrderByDescending<TValue>(Func<T, TValue>)`).
    fn order_by_descending<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T>;

    /// In descending order of the values of `selector`, compared by `comparer`
    /// (`OrderByDescending<TValue>(Func<T, TValue>, IComparer<TValue>)`).
    fn order_by_descending_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T>;
}
