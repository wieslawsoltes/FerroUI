//! `IGroup<TObject, TGroup>` (`List/IGroup.cs`).

use super::IObservableList;
use std::rc::Rc;

/// A group of the items of a list that share a group key, with the list of those items
/// (`IGroup<TObject, TGroup>`): the items of the change sets of `GroupOn` and `GroupOnProperty`.
///
/// Deviation (DEVIATIONS.md, DynamicData): the change sets of groups hold the concrete
/// [`Group`](super::Group), which implements this trait, rather than the interface.
pub trait IGroup<T, G> {
    /// The group key (`GroupKey`).
    fn group_key(&self) -> &G;

    /// The items of the group, as a list that can be observed (`List`).
    fn list(&self) -> Rc<dyn IObservableList<T>>;
}
