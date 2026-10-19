//! `DeferUntilLoaded<TObject, TKey>` (`Cache/Internal/DeferUntilLoaded.cs`).

use super::status_monitor;
use crate::cache::{ChangeSet, IObservableCache, ObservableCacheEx};
use crate::kernel::ConnectionStatus;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// The change sets of `source` from its first one (`DeferUntilLoaded(IObservable<IChangeSet>)`).
pub(crate) fn defer_until_loaded<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    status_monitor(source.clone())
        .where_(|status: &ConnectionStatus| *status == ConnectionStatus::Loaded)
        .take(1)
        .select(|_| ChangeSet::<T, K>::empty())
        .concat(source)
        .not_empty()
}

/// The change sets of `source` once it has items (`DeferUntilLoaded(IObservableCache)`).
pub(crate) fn defer_until_loaded_cache<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: IObservableCache<T, K> + ?Sized>(source: &C) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    source
        .count_changed()
        .where_(|count: &usize| *count != 0)
        .take(1)
        .select(|_| ChangeSet::<T, K>::empty())
        .concat(source.connect())
        .not_empty()
}
