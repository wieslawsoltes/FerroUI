use super::{CollectionChangedHandler, FerroList};
use crate::data::model::{INotifyCollectionChanged, INotifyPropertyChanged};
use std::rc::Rc;

/// A read-only notifying list.
///
/// The members of the read-only list contract it extends (count, indexer,
/// enumeration) are part of the trait, together with the typed form of the
/// collection changed event, which carries the items;
/// [`INotifyCollectionChanged`] is its untyped form.
pub trait IFerroReadOnlyList<T>: INotifyCollectionChanged + INotifyPropertyChanged {
    /// The number of items in the list.
    fn count(&self) -> usize;

    /// The item at `index`. Panics if out of range.
    fn get(&self, index: usize) -> T;

    /// The items, as a snapshot that stays unchanged if the list is
    /// modified (enumeration).
    fn snapshot(&self) -> Rc<Vec<T>>;

    /// Subscribes to changes. Returns a token for
    /// [`remove_collection_changed`](Self::remove_collection_changed).
    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64;

    /// Unsubscribes a handler.
    fn remove_collection_changed(&self, token: u64) -> bool;
}

impl<T: Clone> IFerroReadOnlyList<T> for FerroList<T> {
    fn count(&self) -> usize {
        FerroList::count(self)
    }

    fn get(&self, index: usize) -> T {
        FerroList::get(self, index)
    }

    fn snapshot(&self) -> Rc<Vec<T>> {
        FerroList::snapshot(self)
    }

    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        FerroList::add_collection_changed(self, handler)
    }

    fn remove_collection_changed(&self, token: u64) -> bool {
        FerroList::remove_collection_changed(self, token)
    }
}
