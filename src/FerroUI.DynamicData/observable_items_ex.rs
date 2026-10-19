//! The operators of plain sequences that make change sets: `ToObservableChangeSet`
//! (`Cache/ObservableCacheEx.ToObservableChangeSet.cs`, `List/ObservableListEx.ToObservableChangeSet.cs`)
//! and `EditDiff` of a sequence of collections (`Cache/ObservableCacheEx.EditDiff.cs`).
//!
//! Upstream's overloads differ by the type of the sequence (`IObservable<T>` or
//! `IObservable<IEnumerable<T>>`) and by optional arguments. Here a sequence of items has the
//! methods of [`ObservableItemsEx`], and a sequence of collections those of
//! [`ObservableSequencesEx`] (the `_many` methods); the cache forms take the suffix `_cache`; the
//! optional arguments are the `_with` form (`expire_after`, `limit_size_to` of `None` for upstream's
//! `-1`, and the scheduler), with the default scheduler when `None`.

use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// The expiration of an item: the time after which it is removed, `None` for never.
pub type ExpireAfterSelector<T> = Rc<dyn Fn(&T) -> Option<Duration>>;

/// The operators of sequences of items (`ToObservableChangeSet(IObservable<T>, ..)`), implemented
/// for `Rc<dyn IObservable<T>>`.
pub trait ObservableItemsEx<T: Clone + 'static> {
    /// The sequence as the handle the operators take.
    fn item_sequence(&self) -> Rc<dyn IObservable<T>>;

    /// The items as additions to a list (`ToObservableChangeSet<T>(IObservable<T>, IScheduler? =
    /// null)`): the first change set comes at the subscription, empty if need be.
    fn to_observable_change_set(&self) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>>
    where
        T: PartialEq,
    {
        crate::list::internal::to_observable_change_set_of_items(self.item_sequence(), None, None, None)
    }

    /// [`to_observable_change_set`](Self::to_observable_change_set), whose items expire after the
    /// time that `expire_after` gives and whose oldest beyond `limit_size_to` are removed, timed on
    /// `scheduler` (`ToObservableChangeSet<T>(IObservable<T>, Func<T, TimeSpan?>?, int,
    /// IScheduler?)` and the shorter overloads).
    fn to_observable_change_set_with(
        &self,
        expire_after: Option<ExpireAfterSelector<T>>,
        limit_size_to: Option<usize>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>>
    where
        T: PartialEq,
    {
        crate::list::internal::to_observable_change_set_of_items(self.item_sequence(), expire_after, limit_size_to, scheduler)
    }

    /// The items as additions or updates of a cache, keyed by `key_selector`
    /// (`ToObservableChangeSet<TObject, TKey>(IObservable<TObject>, Func<TObject, TKey>, ..)`).
    fn to_observable_change_set_cache<K: Eq + Hash + Clone + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        crate::cache::internal::to_observable_change_set_of_items(self.item_sequence(), Rc::new(key_selector), None, None, None)
    }

    /// [`to_observable_change_set_cache`](Self::to_observable_change_set_cache) with the
    /// expiration, the limit of size and the scheduler (`ToObservableChangeSet(IObservable<TObject>,
    /// Func<TObject, TKey>, Func<TObject, TimeSpan?>? = null, int limitSizeTo = -1, IScheduler? =
    /// null)`).
    fn to_observable_change_set_cache_with<K: Eq + Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
        expire_after: Option<ExpireAfterSelector<T>>,
        limit_size_to: Option<usize>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        crate::cache::internal::to_observable_change_set_of_items(self.item_sequence(), Rc::new(key_selector), expire_after, limit_size_to, scheduler)
    }
}

impl<T: Clone + 'static> ObservableItemsEx<T> for Rc<dyn IObservable<T>> {
    fn item_sequence(&self) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}

/// The operators of sequences of collections of items (`ToObservableChangeSet(IObservable<
/// IEnumerable<T>>, ..)` and `EditDiff(IObservable<IEnumerable<TObject>>, ..)`), implemented for
/// `Rc<dyn IObservable<I>>` of every collection `I`.
pub trait ObservableSequencesEx<T: Clone + 'static> {
    /// The items of each collection as additions to a list (`ToObservableChangeSet<T>(
    /// IObservable<IEnumerable<T>>, IScheduler? = null)`).
    fn to_observable_change_set_many(&self) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>>
    where
        T: PartialEq;

    /// [`to_observable_change_set_many`](Self::to_observable_change_set_many) with the expiration,
    /// the limit of size and the scheduler.
    fn to_observable_change_set_many_with(
        &self,
        expire_after: Option<ExpireAfterSelector<T>>,
        limit_size_to: Option<usize>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>>
    where
        T: PartialEq;

    /// The items of each collection as additions or updates of a cache, keyed by `key_selector`
    /// (`ToObservableChangeSet<TObject, TKey>(IObservable<IEnumerable<TObject>>, ..)`).
    fn to_observable_change_set_cache_many<K: Eq + Hash + Clone + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>;

    /// [`to_observable_change_set_cache_many`](Self::to_observable_change_set_cache_many) with the
    /// expiration, the limit of size and the scheduler.
    fn to_observable_change_set_cache_many_with<K: Eq + Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
        expire_after: Option<ExpireAfterSelector<T>>,
        limit_size_to: Option<usize>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>;

    /// The changes of a cache, keyed by `key_selector`, that holds the latest collection: each
    /// collection edits it by difference, items compared by `PartialEq`; the cache ends with the
    /// sequence (`EditDiff(IObservable<IEnumerable<TObject>>, Func<TObject, TKey>,
    /// IEqualityComparer? = null)`).
    fn edit_diff<K: Eq + Hash + Clone + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>
    where
        T: PartialEq;

    /// [`edit_diff`](Self::edit_diff) with the equality of `equality_comparer`.
    fn edit_diff_with_comparer<K: Eq + Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
        equality_comparer: impl Fn(&T, &T) -> bool + 'static,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>;
}

impl<T: Clone + 'static, I: IntoIterator<Item = T> + 'static> ObservableSequencesEx<T> for Rc<dyn IObservable<I>> {
    fn to_observable_change_set_many(&self) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>>
    where
        T: PartialEq,
    {
        crate::list::internal::to_observable_change_set(self.clone(), None, None, None)
    }

    fn to_observable_change_set_many_with(
        &self,
        expire_after: Option<ExpireAfterSelector<T>>,
        limit_size_to: Option<usize>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>>
    where
        T: PartialEq,
    {
        crate::list::internal::to_observable_change_set(self.clone(), expire_after, limit_size_to, scheduler)
    }

    fn to_observable_change_set_cache_many<K: Eq + Hash + Clone + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        crate::cache::internal::to_observable_change_set(self.clone(), Rc::new(key_selector), None, None, None)
    }

    fn to_observable_change_set_cache_many_with<K: Eq + Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
        expire_after: Option<ExpireAfterSelector<T>>,
        limit_size_to: Option<usize>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        crate::cache::internal::to_observable_change_set(self.clone(), Rc::new(key_selector), expire_after, limit_size_to, scheduler)
    }

    fn edit_diff<K: Eq + Hash + Clone + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        crate::cache::internal::edit_diff_change_set(self.clone(), Rc::new(key_selector), Rc::new(|a: &T, b: &T| a == b))
    }

    fn edit_diff_with_comparer<K: Eq + Hash + Clone + 'static>(
        &self,
        key_selector: impl Fn(&T) -> K + 'static,
        equality_comparer: impl Fn(&T, &T) -> bool + 'static,
    ) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        crate::cache::internal::edit_diff_change_set(self.clone(), Rc::new(key_selector), Rc::new(equality_comparer))
    }
}
