//! `MergeChangeSets` (`List/ObservableListEx.MergeChangeSets.cs`): the change sets of several lists
//! merged into those of one.
//!
//! As for caches, the kinds of sources are traits of their own: a sequence with another one or
//! others (provided methods of [`ObservableListEx`](crate::list::ObservableListEx)), an enumerable
//! of sequences ([`ListChangeSetSourcesEx`]) and a sequence of sequences
//! ([`ObservableListChangeSetsEx`]). The default equality of the items, which the copies of the
//! lists use, is `PartialEq`.
//!
//! Not ported: the forms on an `IObservableList` of sequences and on a sequence of change sets of
//! sequences (`IObservableList<IObservable<IChangeSet>>`, `IObservable<IChangeSet<IObservable<
//! IChangeSet>>>`): the items of a list are compared, and a sequence has no equality.

use crate::list::internal::merge_change_sets;
use crate::list::ChangeSet;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::IObservable;
use std::rc::Rc;

macro_rules! merge_change_sets_methods {
    () => {
        /// The change sets of the sequence and of `other`, merged; it completes when both have
        /// completed (`MergeChangeSets(IObservable<IChangeSet<TObject>>, IObservable<IChangeSet<
        /// TObject>>, IEqualityComparer<TObject>? = null, IScheduler? = null, bool completable =
        /// true)`).
        fn merge_change_sets(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::ops::merge_change_sets::merge_sources(vec![self.change_sets(), other], None, None, true)
        }

        /// [`merge_change_sets`](Self::merge_change_sets) with every argument: the equality of the
        /// items (`PartialEq` when `None`), the scheduler that enumerates the two sequences, and
        /// whether the merge completes after them.
        fn merge_change_sets_with(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&T, &T) -> bool>>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
            completable: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            crate::list::ops::merge_change_sets::merge_sources(vec![self.change_sets(), other], equality_comparer, scheduler, completable)
        }

        /// The change sets of the sequence and of `others`, merged
        /// (`MergeChangeSets(IObservable<IChangeSet<TObject>>, IEnumerable<IObservable<IChangeSet<
        /// TObject>>>, ..)`).
        fn merge_change_sets_many(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            let mut sources = vec![self.change_sets()];
            sources.extend(others);
            crate::list::ops::merge_change_sets::merge_sources(sources, None, None, true)
        }

        /// [`merge_change_sets_many`](Self::merge_change_sets_many) with every argument.
        fn merge_change_sets_many_with(
            &self,
            others: Vec<std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>>>,
            equality_comparer: Option<std::rc::Rc<dyn Fn(&T, &T) -> bool>>,
            scheduler: Option<std::rc::Rc<dyn ferroui_reactive::concurrency::IScheduler>>,
            completable: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<T>>> {
            let mut sources = vec![self.change_sets()];
            sources.extend(others);
            crate::list::ops::merge_change_sets::merge_sources(sources, equality_comparer, scheduler, completable)
        }
    };
}
pub(crate) use merge_change_sets_methods;

/// The merge of an enumerable of sequences (`new MergeChangeSets(IEnumerable<..>, ..)`).
pub(crate) fn merge_sources<T: Clone + PartialEq + 'static>(
    sources: Vec<Rc<dyn IObservable<ChangeSet<T>>>>,
    equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>,
    scheduler: Option<Rc<dyn IScheduler>>,
    completable: bool,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    merge_change_sets(
        crate::cache::internal::sources_observable(sources, completable, scheduler),
        equality_comparer.unwrap_or_else(crate::cache::internal::default_equality_comparer),
    )
}

/// `MergeChangeSets` of an enumerable of sequences of change sets of lists
/// (`MergeChangeSets(this IEnumerable<IObservable<IChangeSet<TObject>>>, ..)`): the sequences are
/// enumerated when the merge is subscribed, and the merge completes after them unless
/// `completable` is false.
pub trait ListChangeSetSourcesEx<T> {
    /// The change sets of the sequences, merged (`MergeChangeSets()`).
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>>;

    /// With every argument: the equality of the items (`PartialEq` when `None`), the scheduler that
    /// enumerates the sequences, and whether the merge completes after them
    /// (`MergeChangeSets(IEqualityComparer<TObject>?, IScheduler?, bool)`).
    fn merge_change_sets_with(
        &self,
        equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>,
        scheduler: Option<Rc<dyn IScheduler>>,
        completable: bool,
    ) -> Rc<dyn IObservable<ChangeSet<T>>>;
}

impl<T: Clone + PartialEq + 'static> ListChangeSetSourcesEx<T> for [Rc<dyn IObservable<ChangeSet<T>>>] {
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        merge_sources(self.to_vec(), None, None, true)
    }

    fn merge_change_sets_with(
        &self,
        equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>,
        scheduler: Option<Rc<dyn IScheduler>>,
        completable: bool,
    ) -> Rc<dyn IObservable<ChangeSet<T>>> {
        merge_sources(self.to_vec(), equality_comparer, scheduler, completable)
    }
}

/// `MergeChangeSets` of a sequence of sequences of change sets of lists
/// (`MergeChangeSets(this IObservable<IObservable<IChangeSet<TObject>>>, IEqualityComparer<
/// TObject>? = null)`).
pub trait ObservableListChangeSetsEx<T> {
    /// The change sets of the sequences, merged (`MergeChangeSets()`).
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>>;

    /// With the equality of the items (`MergeChangeSets(IEqualityComparer<TObject>)`).
    fn merge_change_sets_with_comparer(&self, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T>>>;
}

impl<T: Clone + PartialEq + 'static> ObservableListChangeSetsEx<T> for Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T>>>>> {
    fn merge_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        merge_change_sets(self.clone(), crate::cache::internal::default_equality_comparer())
    }

    fn merge_change_sets_with_comparer(&self, equality_comparer: impl Fn(&T, &T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T>>> {
        merge_change_sets(self.clone(), Rc::new(equality_comparer))
    }
}
