//! The operators of the sequences of sorted change sets (the extension methods of
//! `ObservableCacheEx` that take `IObservable<ISortedChangeSet<TObject, TKey>>`), and the
//! aggregators of the sorted, paged, virtual and contextual change sets (`Cache/Tests/TestEx.cs`).
//!
//! A sorted stream is also a stream of change sets, so it has every operator of
//! [`ObservableCacheEx`](super::ObservableCacheEx) as well; the methods here have names of their
//! own so that both traits can be in scope (`as_sorted_aggregator` beside `as_aggregator`,
//! `top_sorted` beside `top`).

use super::testing::TypedChangeSetAggregator;
use super::{ChangeSetWithContext, ISortedChangeSet, SortedChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::hash::Hash;
use std::rc::Rc;

/// The operators of sorted change sets (`ObservableCacheEx` over `ISortedChangeSet`), for every
/// sequence of sorted, paged or virtual change sets `C`.
pub trait SortedObservableCacheEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: ISortedChangeSet<T, K>> {
    /// The sequence itself.
    fn sorted_source(&self) -> Rc<dyn IObservable<C>>;

    /// The sequence as plain sorted change sets (the upcast of a paged or virtual change set to
    /// `ISortedChangeSet<TObject, TKey>`).
    fn sorted_change_sets(&self) -> Rc<dyn IObservable<SortedChangeSet<T, K>>>;

    /// Records the change sets, the items and the end of the sequence, for tests
    /// (`TestEx.AsAggregator` of sorted, paged and virtual change sets, which make a
    /// `SortedChangeSetAggregator`, a `PagedChangeSetAggregator` or a
    /// `VirtualChangeSetAggregator`).
    fn as_sorted_aggregator(&self) -> TypedChangeSetAggregator<T, K, C> {
        TypedChangeSetAggregator::new(self.sorted_source())
    }

    super::ops::treat_moves_as_remove_add::treat_moves_as_remove_add_methods!();
    super::ops::update_index::update_index_methods!();
    super::ops::virtualise_and_page::virtualise_and_page_sorted_methods!();
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: ISortedChangeSet<T, K>> SortedObservableCacheEx<T, K, C> for Rc<dyn IObservable<C>> {
    fn sorted_source(&self) -> Rc<dyn IObservable<C>> {
        self.clone()
    }

    fn sorted_change_sets(&self) -> Rc<dyn IObservable<SortedChangeSet<T, K>>> {
        if let Some(sorted) = (self as &dyn Any).downcast_ref::<Rc<dyn IObservable<SortedChangeSet<T, K>>>>() {
            return sorted.clone();
        }
        self.select(|changes: C| changes.to_sorted_change_set())
    }
}

/// The aggregator of the change sets with a context (`TestEx.AsAggregator` of
/// `IObservable<IChangeSet<TObject, TKey, TContext>>`).
pub trait ContextObservableCacheEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static, X: Clone + 'static> {
    /// Records the change sets, the items and the end of the sequence, for tests
    /// (`ChangeSetAggregator<TObject, TKey, TContext>`).
    fn as_context_aggregator(&self) -> TypedChangeSetAggregator<T, K, ChangeSetWithContext<T, K, X>>;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, X: Clone + 'static> ContextObservableCacheEx<T, K, X> for Rc<dyn IObservable<ChangeSetWithContext<T, K, X>>> {
    fn as_context_aggregator(&self) -> TypedChangeSetAggregator<T, K, ChangeSetWithContext<T, K, X>> {
        TypedChangeSetAggregator::new(self.clone())
    }
}
