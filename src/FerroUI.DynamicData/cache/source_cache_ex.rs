//! `SourceCacheEx` (`Cache/SourceCacheEx.cs`), with the operators and edit conveniences of
//! `ObservableCacheEx` whose receiver is a cache rather than a sequence of change sets: those on
//! `IObservableCache` (`DeferUntilLoaded`, `WatchValue`) and those on `ISourceCache`
//! (`AddOrUpdate`, `Clear`, `EditDiff`, `ExpireAfter`, `LimitSizeTo`, `PopulateFrom`, `Refresh`,
//! `Remove`, `RemoveKey`, `RemoveKeys`).
//!
//! The trait is implemented for every cache; the methods of a source cache require it to be one
//! (`where Self: ISourceCache<T, K>`), and those that keep the cache in a subscription require a
//! handle that clones (`SourceCache` and its kin are handles).

use super::{ChangeSet, IConnectableCache, IObservableCache};
use std::hash::Hash;
use std::rc::Rc;

/// The operators and edit conveniences of caches (`SourceCacheEx`, and the extension methods of
/// `ObservableCacheEx` on `IObservableCache` and `ISourceCache`).
pub trait SourceCacheEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static>: IObservableCache<T, K> {
    /// The items and their changes, each item converted by `converter`
    /// (`Cast(IObservableCache<TSource, TKey>, Func<TSource, TDestination>)`).
    fn cast_cache<U: Clone + 'static>(&self, converter: impl Fn(&T) -> U + 'static) -> Rc<dyn ferroui_system::IObservable<ChangeSet<U, K>>> {
        super::internal::cast(IConnectableCache::connect(self), Rc::new(converter))
    }

    super::ops::add_or_update::add_or_update_source_methods!();
    super::ops::clear::clear_source_methods!();
    super::ops::defer_until_loaded::defer_until_loaded_source_methods!();
    super::ops::edit_diff::edit_diff_source_methods!();
    super::ops::expire_after::expire_after_source_methods!();
    super::ops::limit_size_to::limit_size_to_source_methods!();
    super::ops::populate_from::populate_from_source_methods!();
    super::ops::refresh::refresh_source_methods!();
    super::ops::remove::remove_source_methods!();
    super::ops::remove_key::remove_key_source_methods!();
    super::ops::remove_keys::remove_keys_source_methods!();
    super::ops::watch_value::watch_value_source_methods!();
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: IObservableCache<T, K> + ?Sized> SourceCacheEx<T, K> for C {}
