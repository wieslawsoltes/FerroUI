//! `ObservableCacheEx` (`Cache/ObservableCacheEx*.cs`): the operators of the sequences of change
//! sets of caches.
//!
//! Each upstream operator is a provided method, from the `*_methods!` macro of its module in
//! `ops/`. The trait is implemented for every sequence of change sets of any form ([`IChangeSet`]),
//! as upstream's extension methods take the interface (docs/porting/reactive.md, section 5.4).
//!
//! Naming (DEVIATIONS.md, DynamicData): an operator whose name is that of an operator of
//! `ferroui_reactive::ObservableExt`, which applies to the same sequences, takes the suffix `_cache`
//! (`and_cache`, `cast_cache`, `of_type_cache`, `switch_cache`, `count_cache`, `is_empty_cache`,
//! `sum_cache`), so that both traits can be in scope. The LINQ-named aliases of
//! `ObservableCacheAlias` (`Select`, `Where`, `SelectMany`) are not ported for the same reason.

use super::{ChangeSet, IChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::hash::Hash;
use std::rc::Rc;

/// The operators of caches (`ObservableCacheEx`).
pub trait ObservableCacheEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
    /// The sequence as plain change sets (the upcast of a sorted, paged or contextual change set to
    /// `IChangeSet<TObject, TKey>`).
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>;

    super::ops::as_aggregator::as_aggregator_methods!();
    super::ops::as_observable_cache::as_observable_cache_methods!();
    super::ops::collect_update_stats::collect_update_stats_methods!();
    super::ops::filter::filter_methods!();
    super::ops::not_empty::not_empty_methods!();
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: IChangeSet<T, K>> ObservableCacheEx<T, K> for Rc<dyn IObservable<C>> {
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        if let Some(plain) = (self as &dyn Any).downcast_ref::<Rc<dyn IObservable<ChangeSet<T, K>>>>() {
            return plain.clone();
        }
        self.select(|changes: C| changes.change_set().clone())
    }
}
