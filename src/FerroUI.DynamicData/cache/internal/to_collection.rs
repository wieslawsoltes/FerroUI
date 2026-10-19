//! The bodies of `ToCollection` and `ToSortedCollection` of caches
//! (`Cache/ObservableCacheEx.ToCollection.cs`, `Cache/ObservableCacheEx.ToSortedCollection.cs`):
//! `QueryWhenChanged` with a selector of the items.

use crate::cache::internal::Cache;
use crate::cache::{ChangeSet, ICache, IQuery};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The items of the cache after each change set, chosen by `selector` (`QueryWhenChanged(query =>
/// ..)` without a trigger: a cache per subscription, cloned from each change set).
pub(crate) fn query_items<T: Clone + 'static, K: Eq + Hash + Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    selector: Rc<dyn Fn(Vec<T>) -> R>,
) -> Rc<dyn IObservable<R>> {
    Observable::defer(move || {
        let cache: RefCell<Option<Cache<T, K>>> = RefCell::new(None);
        let selector = selector.clone();
        source.select(move |changes: ChangeSet<T, K>| {
            let mut cache = cache.borrow_mut();
            let cache = cache.get_or_insert_with(|| Cache::with_capacity(changes.count()));
            cache.clone_changes(&changes);
            selector(cache.items())
        })
    })
}
