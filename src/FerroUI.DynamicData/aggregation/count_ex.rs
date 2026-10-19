//! `CountEx` (`Aggregation/CountEx.cs`): the count of the items, and whether there are none.
//!
//! The operators are methods of [`AggregationCacheEx`](super::AggregationCacheEx)
//! (`count_cache`, `is_empty_cache`, `is_not_empty`), [`AggregationListEx`](super::AggregationListEx)
//! (`count_list`, `is_empty_list`, `is_not_empty`) and
//! [`AggregateChangeSetEx`](super::AggregateChangeSetEx) (`count_aggregate`): `Count` and `IsEmpty`
//! are names of operators of `ferroui_reactive::ObservableExt`, which applies to the same
//! sequences. The overload for `IDistinctChangeSet<T>` is `count_cache`, as a distinct change set
//! is a change set of a cache.

use super::{accumulate, AggregateChangeSets};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Count<TObject>(IObservable<IAggregateChangeSet<TObject>>)`: the count of the items after each
/// change set.
///
/// Deviation (DEVIATIONS.md, DynamicData): the count is a `usize`, the type of counts in Rust,
/// where upstream's is an `int`.
pub(crate) fn count<T: 'static>(source: AggregateChangeSets<T>) -> Rc<dyn IObservable<usize>> {
    accumulate(source, 0usize, |_: &T| 1usize, |current: usize, increment: usize| current + increment, |current: usize, increment: usize| current.saturating_sub(increment))
}

/// `IsEmpty`: true at once, then whether the count is zero after each change set.
pub(crate) fn is_empty<T: 'static>(source: AggregateChangeSets<T>) -> Rc<dyn IObservable<bool>> {
    count(source).start_with([0usize]).select(|count: usize| count == 0)
}

/// `IsNotEmpty`: false at once, then whether the count is above zero after each change set.
pub(crate) fn is_not_empty<T: 'static>(source: AggregateChangeSets<T>) -> Rc<dyn IObservable<bool>> {
    count(source).start_with([0usize]).select(|count: usize| count > 0)
}
