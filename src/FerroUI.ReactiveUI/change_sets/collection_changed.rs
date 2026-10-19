//! `CollectionChanged` (`ReactiveUI.Core/ChangeSets/CollectionChanged.cs`).

use ferroui_system::collections::specialized::CollectionChange;

/// One `CollectionChanged` notification of a collection, with the collection that raised it
/// (`CollectionChanged`): the element of [`observe_collection_changes`](super::CollectionChangedExtensions::observe_collection_changes).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's sender is an `object` and its arguments are
/// the `NotifyCollectionChangedEventArgs` object, both compared by reference. The sender here is
/// the handle of the collection, of its own type, and the arguments are the untyped description of
/// the change that `INotifyCollectionChanged` raises in `ferroui-system` ([`CollectionChange`]),
/// compared by value.
#[derive(Clone, Debug)]
pub struct CollectionChanged<S> {
    sender: S,
    event_args: CollectionChange,
}

impl<S> CollectionChanged<S> {
    /// A notification of `sender` (`CollectionChanged(object?, NotifyCollectionChangedEventArgs)`).
    pub fn new(sender: S, event_args: CollectionChange) -> Self {
        Self { sender, event_args }
    }

    /// The collection that raised the event (`Sender`).
    pub fn sender(&self) -> &S {
        &self.sender
    }

    /// The description of the change (`EventArgs`).
    pub fn event_args(&self) -> &CollectionChange {
        &self.event_args
    }
}

impl<S: PartialEq> PartialEq for CollectionChanged<S> {
    /// The same sender and the same change (`Equals(CollectionChanged)`).
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender && self.event_args == other.event_args
    }
}
