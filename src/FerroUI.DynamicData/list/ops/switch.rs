//! `Switch` (`List/ObservableListEx.Switch.cs`): the change sets of the latest of a sequence of
//! lists.
//!
//! Deviation (DEVIATIONS.md, DynamicData): the sources are a sequence of
//! [`ListChangeSetSource`]s, so that the overloads on a sequence of lists
//! (`IObservable<IObservableList<T>>`) and on a sequence of sequences of change sets
//! (`IObservable<IObservable<IChangeSet<T>>>`) are one method of [`SwitchListEx`]. Naming: it is
//! `switch_list`, as `switch` would hide `ObservableExt::switch`.

use super::combine::ListChangeSetSource;
use crate::list::internal::switch_list_change_sets;
use crate::list::ChangeSet;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Switch` of a sequence of lists or of sequences of change sets.
pub trait SwitchListEx {
    /// The type of the items.
    type Item: Clone + PartialEq + 'static;

    /// The change sets of the latest source: each new source clears the items of the previous
    /// one, then delivers its own; the result completes when the sequence and the latest source
    /// have completed, and an error of either ends it (`Switch(IObservable<IObservableList<T>>)`,
    /// `Switch(IObservable<IObservable<IChangeSet<T>>>)`).
    fn switch_list(&self) -> Rc<dyn IObservable<ChangeSet<Self::Item>>>;
}

impl<S: ListChangeSetSource> SwitchListEx for Rc<dyn IObservable<S>> {
    type Item = S::Item;

    fn switch_list(&self) -> Rc<dyn IObservable<ChangeSet<S::Item>>> {
        // `sources.Select(list => list.Connect()).Switch()` for the lists.
        switch_list_change_sets(self.select(|source: S| source.source_change_sets()))
    }
}
