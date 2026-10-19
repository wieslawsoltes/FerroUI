//! `ComparerChainingExtensions` (`ReactiveUI.Core/Comparers/ComparerChainingExtensions.cs`).

use super::{ChainedComparer, Comparer};
use std::rc::Rc;

/// Chains a comparison onto a parent comparer (`ComparerChainingExtensions`): the result sorts by
/// the parent first, then, where the parent finds two elements equal, by the values of a selector.
///
/// It is implemented for a comparer and for an optional one (upstream's parent may be null, which
/// sorts by the selector alone).
pub trait ComparerChainingExtensions<T> {
    /// Then in ascending order of the values of `selector`, compared by their `Ord`
    /// (`ThenBy<TValue>(Func<T, TValue>)`).
    fn then_by<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T>;

    /// Then in ascending order of the values of `selector`, compared by `comparer`
    /// (`ThenBy<TValue>(Func<T, TValue>, IComparer<TValue>)`).
    fn then_by_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T>;

    /// Then in descending order of the values of `selector`, compared by their `Ord`
    /// (`ThenByDescending<TValue>(Func<T, TValue>)`).
    fn then_by_descending<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T>;

    /// Then in descending order of the values of `selector`, compared by `comparer`
    /// (`ThenByDescending<TValue>(Func<T, TValue>, IComparer<TValue>)`).
    fn then_by_descending_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T>;
}

impl<T: 'static> ComparerChainingExtensions<T> for Option<Comparer<T>> {
    fn then_by<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        self.then_by_with(selector, Rc::new(|x: &V, y: &V| x.cmp(y)))
    }

    fn then_by_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        ChainedComparer::new(self.clone(), move |x: &T, y: &T| comparer(&selector(x), &selector(y))).into_comparer()
    }

    fn then_by_descending<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        self.then_by_descending_with(selector, Rc::new(|x: &V, y: &V| x.cmp(y)))
    }

    fn then_by_descending_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        ChainedComparer::new(self.clone(), move |x: &T, y: &T| comparer(&selector(x), &selector(y)).reverse()).into_comparer()
    }
}

impl<T: 'static> ComparerChainingExtensions<T> for Comparer<T> {
    fn then_by<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        Some(self.clone()).then_by(selector)
    }

    fn then_by_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        Some(self.clone()).then_by_with(selector, comparer)
    }

    fn then_by_descending<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        Some(self.clone()).then_by_descending(selector)
    }

    fn then_by_descending_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        Some(self.clone()).then_by_descending_with(selector, comparer)
    }
}
