//! `OrderedComparer<T>` (`ReactiveUI.Core/Comparers/OrderedComparer{T}.cs`).

use super::{ComparerChainingExtensions, Comparer};
use std::marker::PhantomData;

/// The starting point of a chain of comparers of `T` (`OrderedComparer<T>` and the non-generic
/// `OrderedComparer`): the first key of the order, which `then_by` extends.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's generic and non-generic static classes share
/// the name; here they are the functions of one type, the `For` functions of the non-generic class
/// being [`for_items`](Self::for_items) and [`for_type`](Self::for_type). A comparer is a shared
/// comparison function ([`Comparer`]), as `IComparer<T>` is in the rest of the port.
///
/// ```
/// use ferroui_reactiveui::{ComparerChainingExtensions, OrderedComparer};
///
/// let mut words = vec!["bb", "a", "ab", "b"];
/// let comparer = OrderedComparer::<&str>::order_by(|word| word.len()).then_by(|word| word.to_string());
/// words.sort_by(|x, y| comparer(x, y));
/// assert_eq!(words, vec!["a", "b", "ab", "bb"]);
/// ```
pub struct OrderedComparer<T>(PhantomData<fn(&T)>);

impl<T: 'static> OrderedComparer<T> {
    /// In ascending order of the values of `selector`, compared by their `Ord`
    /// (`OrderBy<TValue>(Func<T, TValue>)`).
    pub fn order_by<V: Ord + 'static>(selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        None::<Comparer<T>>.then_by(selector)
    }

    /// In ascending order of the values of `selector`, compared by `comparer`
    /// (`OrderBy<TValue>(Func<T, TValue>, IComparer<TValue>)`).
    pub fn order_by_with<V: 'static>(selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        None::<Comparer<T>>.then_by_with(selector, comparer)
    }

    /// In descending order of the values of `selector`, compared by their `Ord`
    /// (`OrderByDescending<TValue>(Func<T, TValue>)`).
    pub fn order_by_descending<V: Ord + 'static>(selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        None::<Comparer<T>>.then_by_descending(selector)
    }

    /// In descending order of the values of `selector`, compared by `comparer`
    /// (`OrderByDescending<TValue>(Func<T, TValue>, IComparer<TValue>)`).
    pub fn order_by_descending_with<V: 'static>(selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        None::<Comparer<T>>.then_by_descending_with(selector, comparer)
    }
}
