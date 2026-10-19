//! `Combine` (`List/ObservableListEx.Combine.cs`): the private overloads behind `And`, `Or`,
//! `Xor` and `Except`, and the kinds of sources they take.
//!
//! Deviation (DEVIATIONS.md, DynamicData): as for caches, upstream's static overloads over a
//! collection of sources are methods of extension traits on those collections:
//! - a sequence with others: provided methods of [`ObservableListEx`](crate::list::ObservableListEx)
//!   (`and_list(others)`, `or(others)`, `xor(others)`, `except(others)`);
//! - a collection of sources (`ICollection<IObservable<IChangeSet<T>>>`): [`ListSourcesCombineEx`],
//!   for a slice or a `Vec`;
//! - an observable list of sources (`IObservableList<IObservable<IChangeSet<T>>>`,
//!   `IObservableList<IObservableList<T>>`, `IObservableList<ISourceList<T>>`):
//!   [`ListSourceListCombineEx`], for a `SourceList` or an `ObservableList`.
//!
//! A source is any [`ListChangeSetSource`]: a sequence of change sets, or a list, whose
//! `Connect()` is its sequence. The combiners count the items of each list in a dictionary, so the
//! items are `Eq + Hash` (upstream's `Dictionary<T, int>` with the default comparer).
//!
//! Naming (DEVIATIONS.md, DynamicData): `And` is `and_list` everywhere, as the method of
//! `ObservableListEx` would hide the join pattern `ObservableExt::and`.

use crate::cache::internal::CombineOperator;
use crate::list::internal::{combiner, dynamic_combiner};
use crate::list::{ChangeSet, IObservableList, ISourceList, ObservableList, SourceList};
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// `Combine(IObservable<IChangeSet<T>>, CombineOperator, params IObservable<IChangeSet<T>>[])`:
/// the sequence first, then the others.
pub(crate) fn combine_with_others<T: Clone + Eq + Hash + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    kind: CombineOperator,
    others: Vec<Rc<dyn IObservable<ChangeSet<T>>>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    // `source.EnumerateOne().Union(others)`: a sequence given twice is combined once.
    let mut items: Vec<Rc<dyn IObservable<ChangeSet<T>>>> = Vec::with_capacity(others.len() + 1);
    for candidate in std::iter::once(source).chain(others) {
        if !items.iter().any(|item| std::ptr::addr_eq(Rc::as_ptr(item), Rc::as_ptr(&candidate))) {
            items.push(candidate);
        }
    }
    combiner(items, kind)
}

/// A source of the change sets of a list: a sequence of change sets, or a list whose `Connect()`
/// is its sequence (the items of upstream's collections of sources).
pub trait ListChangeSetSource: 'static {
    /// The type of the items.
    type Item: Clone + PartialEq + 'static;

    /// The change sets of the source: the sequence itself, or `Connect()` of a list.
    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>;
}

impl<T: Clone + PartialEq + 'static> ListChangeSetSource for Rc<dyn IObservable<ChangeSet<T>>> {
    type Item = T;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.clone()
    }
}

impl<T: Clone + PartialEq + 'static> ListChangeSetSource for SourceList<T> {
    type Item = T;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.connect()
    }
}

impl<T: Clone + PartialEq + 'static> ListChangeSetSource for ObservableList<T> {
    type Item = T;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.connect()
    }
}

impl<T: Clone + PartialEq + 'static> ListChangeSetSource for Rc<dyn IObservableList<T>> {
    type Item = T;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.connect()
    }
}

impl<T: Clone + PartialEq + 'static> ListChangeSetSource for Rc<dyn ISourceList<T>> {
    type Item = T;

    fn source_change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.connect()
    }
}

/// Lists compare by identity (reference equality), so that they can be the items of a list.
impl<T> PartialEq for dyn IObservableList<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Lists compare by identity (reference equality), so that they can be the items of a list.
impl<T: Clone + PartialEq> PartialEq for dyn ISourceList<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// `And`, `Or`, `Xor` and `Except` of a collection of sources (`And(this
/// ICollection<IObservable<IChangeSet<T>>>)`, ..): the sources are subscribed in order when the
/// result is subscribed.
pub trait ListSourcesCombineEx {
    /// The type of the items.
    type Item: Clone + PartialEq + 'static;

    /// The items that are in every source (`And(ICollection<..>)`).
    fn and_list(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;

    /// The items that are in any source (`Or(ICollection<..>)`).
    fn or(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;

    /// The items that are in exactly one source (`Xor(ICollection<..>)`).
    fn xor(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;

    /// The items of the first source that are in none of the others (`Except(ICollection<..>)`).
    fn except(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;
}

/// `Combine(ICollection<IObservable<IChangeSet<T>>>, CombineOperator)`.
fn combine_collection<S: ListChangeSetSource>(sources: &[S], kind: CombineOperator) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
where
    S::Item: Eq + Hash,
{
    combiner(sources.iter().map(ListChangeSetSource::source_change_sets).collect(), kind)
}

impl<S: ListChangeSetSource> ListSourcesCombineEx for [S] {
    type Item = S::Item;

    fn and_list(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
    where
        Self::Item: Eq + Hash,
    {
        combine_collection(self, CombineOperator::And)
    }

    fn or(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
    where
        Self::Item: Eq + Hash,
    {
        combine_collection(self, CombineOperator::Or)
    }

    fn xor(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
    where
        Self::Item: Eq + Hash,
    {
        combine_collection(self, CombineOperator::Xor)
    }

    fn except(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
    where
        Self::Item: Eq + Hash,
    {
        combine_collection(self, CombineOperator::Except)
    }
}

/// `And`, `Or`, `Xor` and `Except` of an observable list of sources (`And(this
/// IObservableList<IObservable<IChangeSet<T>>>)`, `And(this IObservableList<IObservableList<T>>)`,
/// `And(this IObservableList<ISourceList<T>>)`, ..): the result follows the sources as they are
/// added to the list and removed from it.
pub trait ListSourceListCombineEx {
    /// The type of the items.
    type Item: Clone + PartialEq + 'static;

    /// The items that are in every source of the list; none while the list is empty
    /// (`And(IObservableList<..>)`).
    fn and_list(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;

    /// The items that are in any source of the list (`Or(IObservableList<..>)`).
    fn or(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;

    /// The items that are in exactly one source of the list (`Xor(IObservableList<..>)`).
    fn xor(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;

    /// The items of the first source of the list that are in none of the others
    /// (`Except(IObservableList<..>)`).
    fn except(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>
    where
        Self::Item: Eq + Hash;
}

/// `Combine(IObservableList<..>, CombineOperator)`, with `new DynamicCombiner(..).Run()`.
fn combine_list<S: ListChangeSetSource + Clone + PartialEq>(list: &dyn IObservableList<S>, kind: CombineOperator) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
where
    S::Item: Eq + Hash,
{
    dynamic_combiner(list.connect(), Rc::new(|source: &S| source.source_change_sets()), kind)
}

/// The forms of a list of sources, for one type of list.
macro_rules! list_source_list_combine_impl {
    ($list:ident) => {
        impl<S: ListChangeSetSource + Clone + PartialEq> ListSourceListCombineEx for $list<S> {
            type Item = S::Item;

            fn and_list(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
            where
                Self::Item: Eq + Hash,
            {
                combine_list(self, CombineOperator::And)
            }

            fn or(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
            where
                Self::Item: Eq + Hash,
            {
                combine_list(self, CombineOperator::Or)
            }

            fn xor(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
            where
                Self::Item: Eq + Hash,
            {
                combine_list(self, CombineOperator::Xor)
            }

            fn except(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>>
            where
                Self::Item: Eq + Hash,
            {
                combine_list(self, CombineOperator::Except)
            }
        }
    };
}

list_source_list_combine_impl!(SourceList);
list_source_list_combine_impl!(ObservableList);
