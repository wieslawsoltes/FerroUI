//! `Combine` (`Cache/ObservableCacheEx.Combine.cs`): the private overloads behind `And`, `Or`,
//! `Xor` and `Except`, and the kinds of sources they take.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's static overloads over a collection of
//! sources are methods of extension traits on those collections, as for `MergeChangeSets`:
//! - a sequence with others: provided methods of [`ObservableCacheEx`](crate::cache::ObservableCacheEx)
//!   (`and_cache(others)`, `or(others)`, `xor(others)`, `except(others)`);
//! - a collection of sources (`ICollection<IObservable<IChangeSet>>`): [`CacheSourcesCombineEx`],
//!   for a slice or a `Vec`;
//! - an observable list of sources (`IObservableList<IObservable<IChangeSet>>`,
//!   `IObservableList<IObservableCache>`, `IObservableList<ISourceCache>`):
//!   [`CacheSourceListCombineEx`], for a `SourceList` or an `ObservableList`.
//!
//! A source is any [`CacheChangeSetSource`]: a sequence of change sets, or a cache, whose
//! `Connect()` is its sequence.
//!
//! Deviation (DEVIATIONS.md, DynamicData): the combiners keep the item they hold when a source
//! gives the same object again (`ReferenceEquals`). The reference of an item is the object its
//! pointer refers to (`T: Deref`, an `Rc<..>` for the classes of upstream), as
//! `ignore_same_reference_update` does; the combiners of caches require it.
//!
//! Naming (DEVIATIONS.md, DynamicData): `And` is `and_cache` everywhere, as the method of
//! `ObservableCacheEx` would hide the join pattern `ObservableExt::and`.

use crate::cache::internal::{combiner, dynamic_combiner, CombineOperator, SameReference};
use crate::cache::{ChangeSet, IConnectableCache, IObservableCache, ISourceCache, ObservableCache, SourceCache};
use crate::list::{IObservableList, ObservableList, SourceList};
use ferroui_system::IObservable;
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

/// What `ReferenceEquals` is for an item: the same object behind the pointer.
pub(crate) fn same_reference<T: Deref + 'static>() -> SameReference<T> {
    Rc::new(|a: &T, b: &T| std::ptr::addr_eq(Deref::deref(a) as *const T::Target, Deref::deref(b) as *const T::Target))
}

/// `Combine(IObservable<IChangeSet>, CombineOperator, params IObservable<IChangeSet>[])`: the
/// sequence first, then the others.
pub(crate) fn combine_with_others<T: Clone + Deref + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    kind: CombineOperator,
    others: Vec<Rc<dyn IObservable<ChangeSet<T, K>>>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    let mut sources = Vec::with_capacity(others.len() + 1);
    sources.push(source);
    sources.extend(others);
    combiner(sources, kind, same_reference())
}

/// A source of the change sets of a cache: a sequence of change sets, or a cache whose `Connect()`
/// is its sequence (the items of upstream's collections of sources).
pub trait CacheChangeSetSource: 'static {
    /// The type of the items.
    type Item: Clone + 'static;
    /// The type of the keys.
    type Key: Eq + Hash + Clone + 'static;

    /// The change sets of the source: the sequence itself, or `Connect()` of a cache.
    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheChangeSetSource for Rc<dyn IObservable<ChangeSet<T, K>>> {
    type Item = T;
    type Key = K;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.clone()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheChangeSetSource for SourceCache<T, K> {
    type Item = T;
    type Key = K;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.connect()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheChangeSetSource for ObservableCache<T, K> {
    type Item = T;
    type Key = K;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.connect()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheChangeSetSource for Rc<dyn IObservableCache<T, K>> {
    type Item = T;
    type Key = K;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.connect()
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> CacheChangeSetSource for Rc<dyn ISourceCache<T, K>> {
    type Item = T;
    type Key = K;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.connect()
    }
}

/// Caches compare by identity (reference equality), so that they can be the items of a list.
impl<T, K> PartialEq for dyn IObservableCache<T, K> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Caches compare by identity (reference equality), so that they can be the items of a list.
impl<T, K> PartialEq for dyn ISourceCache<T, K> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// `And`, `Or`, `Xor` and `Except` of a collection of sources (`And(this
/// ICollection<IObservable<IChangeSet<TObject, TKey>>>)`, ..): the sources are subscribed in
/// order when the result is subscribed.
pub trait CacheSourcesCombineEx {
    /// The type of the items.
    type Item: Clone + 'static;
    /// The type of the keys.
    type Key: Eq + Hash + Clone + 'static;

    /// The items that are in every source (`And(ICollection<..>)`).
    fn and_cache(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;

    /// The items that are in any source (`Or(ICollection<..>)`).
    fn or(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;

    /// The items that are in exactly one source (`Xor(ICollection<..>)`).
    fn xor(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;

    /// The items of the first source that are in none of the others (`Except(ICollection<..>)`).
    fn except(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;
}

/// `Combine(ICollection<IObservable<IChangeSet>>, CombineOperator)`.
fn combine_collection<S: CacheChangeSetSource>(sources: &[S], kind: CombineOperator) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
where
    S::Item: Deref,
{
    combiner(sources.iter().map(CacheChangeSetSource::source_change_sets).collect(), kind, same_reference())
}

impl<S: CacheChangeSetSource> CacheSourcesCombineEx for [S] {
    type Item = S::Item;
    type Key = S::Key;

    fn and_cache(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
    where
        Self::Item: Deref,
    {
        combine_collection(self, CombineOperator::And)
    }

    fn or(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
    where
        Self::Item: Deref,
    {
        combine_collection(self, CombineOperator::Or)
    }

    fn xor(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
    where
        Self::Item: Deref,
    {
        combine_collection(self, CombineOperator::Xor)
    }

    fn except(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
    where
        Self::Item: Deref,
    {
        combine_collection(self, CombineOperator::Except)
    }
}

/// `And`, `Or`, `Xor` and `Except` of an observable list of sources (`And(this
/// IObservableList<IObservable<IChangeSet<TObject, TKey>>>)`, `And(this
/// IObservableList<IObservableCache<TObject, TKey>>)`, `And(this
/// IObservableList<ISourceCache<TObject, TKey>>)`, ..): the result follows the sources as they are
/// added to the list and removed from it.
pub trait CacheSourceListCombineEx {
    /// The type of the items.
    type Item: Clone + 'static;
    /// The type of the keys.
    type Key: Eq + Hash + Clone + 'static;

    /// The items that are in every source of the list; none while the list is empty
    /// (`And(IObservableList<..>)`).
    fn and_cache(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;

    /// The items that are in any source of the list (`Or(IObservableList<..>)`).
    fn or(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;

    /// The items that are in exactly one source of the list (`Xor(IObservableList<..>)`).
    fn xor(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;

    /// The items of the first source of the list that are in none of the others
    /// (`Except(IObservableList<..>)`).
    fn except(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item, Self::Key>>>
    where
        Self::Item: Deref;
}

/// `Combine(IObservableList<..>, CombineOperator)`, with `new DynamicCombiner(..).Run()`.
fn combine_list<S: CacheChangeSetSource + Clone + PartialEq>(list: &dyn IObservableList<S>, kind: CombineOperator) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
where
    S::Item: Deref,
{
    dynamic_combiner(list.connect(), Rc::new(|source: &S| source.source_change_sets()), kind, same_reference())
}

/// The forms of a list of sources, for one type of list.
macro_rules! cache_source_list_combine_impl {
    ($list:ident) => {
        impl<S: CacheChangeSetSource + Clone + PartialEq> CacheSourceListCombineEx for $list<S> {
            type Item = S::Item;
            type Key = S::Key;

            fn and_cache(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
            where
                Self::Item: Deref,
            {
                combine_list(self, CombineOperator::And)
            }

            fn or(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
            where
                Self::Item: Deref,
            {
                combine_list(self, CombineOperator::Or)
            }

            fn xor(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
            where
                Self::Item: Deref,
            {
                combine_list(self, CombineOperator::Xor)
            }

            fn except(&self) -> Rc<dyn IObservable<ChangeSet<S::Item, S::Key>>>
            where
                Self::Item: Deref,
            {
                combine_list(self, CombineOperator::Except)
            }
        }
    };
}

cache_source_list_combine_impl!(SourceList);
cache_source_list_combine_impl!(ObservableList);
