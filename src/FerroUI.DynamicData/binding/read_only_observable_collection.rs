//! `ReadOnlyObservableCollection<T>` (`System.Collections.ObjectModel`), the read-only view that the
//! `out` forms of `Bind` and `SortAndBind` hand out.
//!
//! Deviation (DEVIATIONS.md, DynamicData): the type belongs to the base library upstream; it lives
//! here until `ferroui-system` has it. It shares the notifications of the collection it wraps
//! rather than raising its own with itself as the sender.

use super::ObservableCollectionExtended;
use ferroui_system::collections::object_model::{CollectionChangedHandler, ObservableCollection};
use ferroui_system::collections::specialized::{CollectionChange, INotifyCollectionChanged};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::Ref;
use std::fmt;
use std::rc::Rc;

/// A read-only view of an observable collection (`ReadOnlyObservableCollection<T>`).
pub struct ReadOnlyObservableCollection<T> {
    collection: ObservableCollection<T>,
}

impl<T> Clone for ReadOnlyObservableCollection<T> {
    fn clone(&self) -> Self {
        Self { collection: self.collection.clone() }
    }
}

impl<T> PartialEq for ReadOnlyObservableCollection<T> {
    /// A view of the same collection.
    fn eq(&self, other: &Self) -> bool {
        self.collection == other.collection
    }
}

impl<T: Clone + PartialEq + fmt::Debug + 'static> fmt::Debug for ReadOnlyObservableCollection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.collection.items().iter()).finish()
    }
}

impl<T: Clone + PartialEq + 'static> ReadOnlyObservableCollection<T> {
    /// The view of `collection` (`ReadOnlyObservableCollection(ObservableCollection<T>)`).
    pub fn new(collection: ObservableCollection<T>) -> Self {
        Self { collection }
    }

    /// The collection that the view shows.
    pub(crate) fn collection(&self) -> &ObservableCollection<T> {
        &self.collection
    }

    /// Adds a handler of the typed change notifications; returns the token that removes it
    /// (`CollectionChanged +=`).
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        self.collection.add_collection_changed(handler)
    }

    /// Removes a handler (`CollectionChanged -=`).
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.collection.remove_collection_changed(token)
    }

    /// The number of items (`Count`).
    pub fn count(&self) -> usize {
        self.collection.count()
    }

    pub fn is_empty(&self) -> bool {
        self.collection.is_empty()
    }

    /// The item at `index` (the indexer).
    pub fn get(&self, index: usize) -> T {
        self.collection.get(index)
    }

    /// The items, borrowed.
    pub fn items(&self) -> Ref<'_, Vec<T>> {
        self.collection.items()
    }

    /// A copy of the items.
    pub fn to_vec(&self) -> Vec<T> {
        self.collection.to_vec()
    }

    /// Whether the collection has `item` (`Contains`).
    pub fn contains(&self, item: &T) -> bool {
        self.collection.contains(item)
    }

    /// The index of `item` (`IndexOf`).
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.collection.index_of(item)
    }
}

impl<T: Clone + PartialEq + 'static> From<ObservableCollection<T>> for ReadOnlyObservableCollection<T> {
    fn from(collection: ObservableCollection<T>) -> Self {
        Self::new(collection)
    }
}

impl<T: Clone + PartialEq + 'static> From<&ObservableCollectionExtended<T>> for ReadOnlyObservableCollection<T> {
    /// The view of the collection (`new ReadOnlyObservableCollection<T>(observableCollectionExtended)`).
    fn from(collection: &ObservableCollectionExtended<T>) -> Self {
        Self::new(collection.collection().clone())
    }
}

impl<T> INotifyCollectionChanged for ReadOnlyObservableCollection<T> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        self.collection.collection_changed()
    }
}

impl<T> INotifyPropertyChanged for ReadOnlyObservableCollection<T> {
    fn property_changed(&self) -> &Event<str> {
        self.collection.property_changed()
    }
}
