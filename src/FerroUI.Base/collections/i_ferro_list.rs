use super::{FerroList, IFerroReadOnlyList};
use std::hash::Hash;

/// A notifying list.
///
/// The members of the list contract it extends (add, insert, remove,
/// search) are part of the trait. Ranges are passed as vectors, so that the
/// trait can be used as an object.
pub trait IFerroList<T>: IFerroReadOnlyList<T> {
    /// Sets the element at the specified index.
    fn set(&self, index: usize, value: T);

    /// Adds an item to the collection.
    fn add(&self, item: T);

    /// Inserts an item at the specified index.
    fn insert(&self, index: usize, item: T);

    /// Removes the item at the specified index.
    fn remove_at(&self, index: usize);

    /// Removes all items from the collection.
    fn clear(&self);

    /// Tests whether the collection contains an item.
    fn contains(&self, item: &T) -> bool;

    /// The index of an item in the collection.
    fn index_of(&self, item: &T) -> Option<usize>;

    /// Removes an item from the collection. Returns true if it was present.
    fn remove(&self, item: &T) -> bool;

    /// Adds multiple items to the collection.
    fn add_range(&self, items: Vec<T>);

    /// Inserts multiple items at the specified index.
    fn insert_range(&self, index: usize, items: Vec<T>);

    /// Moves an item to a new index.
    fn move_item(&self, old_index: usize, new_index: usize);

    /// Moves multiple items to a new index.
    fn move_range(&self, old_index: usize, count: usize, new_index: usize);

    /// Removes multiple items from the collection.
    fn remove_all(&self, items: Vec<T>);

    /// Removes a range of elements from the collection.
    fn remove_range(&self, index: usize, count: usize);
}

impl<T: Clone + Eq + Hash> IFerroList<T> for FerroList<T> {
    fn set(&self, index: usize, value: T) {
        FerroList::set(self, index, value)
    }

    fn add(&self, item: T) {
        FerroList::add(self, item)
    }

    fn insert(&self, index: usize, item: T) {
        FerroList::insert(self, index, item)
    }

    fn remove_at(&self, index: usize) {
        FerroList::remove_at(self, index)
    }

    fn clear(&self) {
        FerroList::clear(self)
    }

    fn contains(&self, item: &T) -> bool {
        FerroList::contains(self, item)
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        FerroList::index_of(self, item)
    }

    fn remove(&self, item: &T) -> bool {
        FerroList::remove(self, item)
    }

    fn add_range(&self, items: Vec<T>) {
        FerroList::add_range(self, items)
    }

    fn insert_range(&self, index: usize, items: Vec<T>) {
        FerroList::insert_range(self, index, items)
    }

    fn move_item(&self, old_index: usize, new_index: usize) {
        FerroList::move_item(self, old_index, new_index)
    }

    fn move_range(&self, old_index: usize, count: usize, new_index: usize) {
        FerroList::move_range(self, old_index, count, new_index)
    }

    fn remove_all(&self, items: Vec<T>) {
        FerroList::remove_all(self, items)
    }

    fn remove_range(&self, index: usize, count: usize) {
        FerroList::remove_range(self, index, count)
    }
}
