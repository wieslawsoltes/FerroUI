//! `MergeChangeSets` (`Cache/ObservableCacheEx.MergeChangeSets.cs`): the change sets of several
//! caches merged into those of one.
//!
//! Deviation (DEVIATIONS.md, DynamicData): the upstream overloads take three kinds of sources,
//! each a trait of its own here, and the overloads with an `IComparer` are the `_ordered_by`
//! methods:
//! - a sequence of change sets with another one or others: provided methods of
//!   [`ObservableCacheEx`](crate::cache::ObservableCacheEx) (`merge_change_sets(other)`,
//!   `merge_change_sets_many(others)`);
//! - an enumerable of sequences: [`CacheChangeSetSourcesEx`], for a slice or a `Vec`;
//! - a sequence of sequences: [`ObservableCacheChangeSetsEx`].
//!
//! Without a comparer, the first value of a key stays until it is removed; with one
//! (`_ordered_by`), the value of a key is the least among the caches. The default equality of the
//! items (`EqualityComparer<TObject>.Default`) is `PartialEq`; `_with_comparer` takes another.

use crate::cache::internal::{default_equality_comparer, merge_change_sets, sources_observable};
use crate::cache::ChangeSet;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::IObservable;
use std::cmp::Ordering;
use std::hash::Hash;
use std::rc::Rc;

macro_rules! merge_change_sets_methods {
    () => {
        /// The change sets of the sequence and of `other`, merged; it completes when both have
        /// completed (`MergeChangeSets(IObservable<IChangeSet>, IObservable<IChangeSet>,
        /// IScheduler? = null, bool completable = true)`).
        fn merge_change_sets(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::cache::ops::merge_change_sets::merge_sources(vec![self.change_sets(), other], None, None, None, true)
        }

        /// [`merge_change_sets`](Self::merge_change_sets) with the equality of the items
        /// (`MergeChangeSets(.., IObservable<IChangeSet>, IEqualityComparer<TObject>, ..)`).
        fn merge_change_sets_with_comparer(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>,
            equality_comparer: impl Fn(&T, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::ops::merge_change_sets::merge_sources_with(vec![self.change_sets(), other], std::rc::Rc::new(equality_comparer), None, None, true)
        }

        /// [`merge_change_sets`](Self::merge_change_sets) where the value of a key is the least by
        /// `comparer` (`MergeChangeSets(.., IObservable<IChangeSet>, IComparer<TObject>, ..)`).
        fn merge_change_sets_ordered_by(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>,
            comparer: std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::cache::ops::merge_change_sets::merge_sources(vec![self.change_sets(), other], None, Some(comparer), None, true)
        }

        /// [`merge_change_sets`](Self::merge_change_sets) with every argument: the equality of the
        /// items (`PartialEq` when `None`), the comparer, the scheduler that enumerates the two
        /// sequences, and whether the merge completes after them
        /// (`MergeChangeSets(.., IObservable<IChangeSet>, IEqualityComparer<TObject>,
        /// IComparer<TObject>, IScheduler?, bool)`).
        fn merge_change_sets_with(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&T, &T) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
            completable: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            crate::cache::ops::merge_change_sets::merge_sources(vec![self.change_sets(), other], equality_comparer, comparer, scheduler, completable)
        }

        /// The change sets of the sequence and of `others`, merged
        /// (`MergeChangeSets(IObservable<IChangeSet>, IEnumerable<IObservable<IChangeSet>>,
        /// IScheduler? = null, bool completable = true)`).
        fn merge_change_sets_many(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            let mut sources = vec![self.change_sets()];
            sources.extend(others);
            crate::cache::ops::merge_change_sets::merge_sources(sources, None, None, None, true)
        }

        /// [`merge_change_sets_many`](Self::merge_change_sets_many) with every argument
        /// (`MergeChangeSets(.., IEnumerable<IObservable<IChangeSet>>, IEqualityComparer<TObject>,
        /// IComparer<TObject>, IScheduler?, bool)`).
        fn merge_change_sets_many_with(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>>,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&T, &T) -> bool>>,
            comparer: Option<std::rc::Rc<dyn Fn(&T, &T) -> std::cmp::Ordering>>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
            completable: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            let mut sources = vec![self.change_sets()];
            sources.extend(others);
            crate::cache::ops::merge_change_sets::merge_sources(sources, equality_comparer, comparer, scheduler, completable)
        }
    };
}
pub(crate) use merge_change_sets_methods;

/// The merge of an enumerable of sequences, with the default equality when none is given.
pub(crate) fn merge_sources<T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static>(
    sources: Vec<Rc<dyn IObservable<ChangeSet<T, K>>>>,
    equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>,
    comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>,
    scheduler: Option<Rc<dyn IScheduler>>,
    completable: bool,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    merge_sources_with(sources, equality_comparer.unwrap_or_else(default_equality_comparer), comparer, scheduler, completable)
}

/// The merge of an enumerable of sequences (`new MergeChangeSets(IEnumerable<..>, ..)`).
pub(crate) fn merge_sources_with<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    sources: Vec<Rc<dyn IObservable<ChangeSet<T, K>>>>,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
    comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>,
    scheduler: Option<Rc<dyn IScheduler>>,
    completable: bool,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    merge_change_sets(sources_observable(sources, completable, scheduler), equality_comparer, comparer)
}

/// `MergeChangeSets` of an enumerable of sequences of change sets of caches
/// (`MergeChangeSets(this IEnumerable<IObservable<IChangeSet<TObject, TKey>>>, ..)`): the
/// sequences are enumerated when the merge is subscribed, and the merge completes after them
/// unless `completable` is false.
pub trait CacheChangeSetSourcesEx<T, K> {
    /// The change sets of the sequences, merged
    /// (`MergeChangeSets(IScheduler? = null, bool completable = true)`).
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq;

    /// With the equality of the items (`MergeChangeSets(IEqualityComparer<TObject>, ..)`).
    fn merge_change_sets_with_comparer(&self, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T, K>>>;

    /// Where the value of a key is the least by `comparer` (`MergeChangeSets(IComparer<TObject>,
    /// ..)`).
    fn merge_change_sets_ordered_by(&self, comparer: Rc<dyn Fn(&T, &T) -> Ordering>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq;

    /// With every argument: the equality of the items (`PartialEq` when `None`), the comparer, the
    /// scheduler that enumerates the sequences, and whether the merge completes after them
    /// (`MergeChangeSets(IEqualityComparer<TObject>, IComparer<TObject>, IScheduler?, bool)`).
    fn merge_change_sets_with(
        &self,
        equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>,
        comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>,
        scheduler: Option<Rc<dyn IScheduler>>,
        completable: bool,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheChangeSetSourcesEx<T, K> for [Rc<dyn IObservable<ChangeSet<T, K>>>] {
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        merge_sources(self.to_vec(), None, None, None, true)
    }

    fn merge_change_sets_with_comparer(&self, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        merge_sources_with(self.to_vec(), Rc::new(equality_comparer), None, None, true)
    }

    fn merge_change_sets_ordered_by(&self, comparer: Rc<dyn Fn(&T, &T) -> Ordering>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        merge_sources(self.to_vec(), None, Some(comparer), None, true)
    }

    fn merge_change_sets_with(
        &self,
        equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>,
        comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>,
        scheduler: Option<Rc<dyn IScheduler>>,
        completable: bool,
    ) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        merge_sources(self.to_vec(), equality_comparer, comparer, scheduler, completable)
    }
}

/// `MergeChangeSets` of a sequence of sequences of change sets of caches
/// (`MergeChangeSets(this IObservable<IObservable<IChangeSet<TObject, TKey>>>, ..)`): each
/// sequence is merged as it arrives; the merge completes when the outer sequence and every inner
/// one have completed.
pub trait ObservableCacheChangeSetsEx<T, K> {
    /// The change sets of the sequences, merged (`MergeChangeSets()`).
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq;

    /// With the equality of the items (`MergeChangeSets(IEqualityComparer<TObject>)`).
    fn merge_change_sets_with_comparer(&self, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T, K>>>;

    /// Where the value of a key is the least by `comparer` (`MergeChangeSets(IComparer<TObject>)`).
    fn merge_change_sets_ordered_by(&self, comparer: Rc<dyn Fn(&T, &T) -> Ordering>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq;

    /// With the equality of the items (`PartialEq` when `None`) and the comparer
    /// (`MergeChangeSets(IEqualityComparer<TObject>, IComparer<TObject>)`).
    fn merge_change_sets_with(&self, equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>, comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ObservableCacheChangeSetsEx<T, K> for Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T, K>>>>> {
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        merge_change_sets(self.clone(), default_equality_comparer(), None)
    }

    fn merge_change_sets_with_comparer(&self, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        merge_change_sets(self.clone(), Rc::new(equality_comparer), None)
    }

    fn merge_change_sets_ordered_by(&self, comparer: Rc<dyn Fn(&T, &T) -> Ordering>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        merge_change_sets(self.clone(), default_equality_comparer(), Some(comparer))
    }

    fn merge_change_sets_with(&self, equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>, comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        merge_change_sets(self.clone(), equality_comparer.unwrap_or_else(default_equality_comparer), comparer)
    }
}
