//! `SourceListEditConvenienceEx` (`List/SourceListEditConvenienceEx.cs`): the edits of one step of
//! source lists.

use super::internal::edit_diff;
use super::{remove_many, replace, IExtendedList, IList, ISourceList};

/// The edit conveniences of source lists (`SourceListEditConvenienceEx`), implemented for every
/// source list.
pub trait SourceListEditConvenienceEx<T: Clone + PartialEq + 'static>: ISourceList<T> {
    /// Adds `item` (`Add`).
    fn add(&self, item: T) {
        let mut item = Some(item);
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(item) = item.take() {
                IList::add(list, item);
            }
        });
    }

    /// Adds the items (`AddRange`).
    fn add_range(&self, items: impl IntoIterator<Item = T>) {
        let mut items = Some(items.into_iter().collect::<Vec<T>>());
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(items) = items.take() {
                IList::add_range(list, items);
            }
        });
    }

    /// Removes every item (`Clear`).
    fn clear(&self) {
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| IList::clear(list));
    }

    /// Makes the list hold `all_items` with the fewest changes, items compared by `PartialEq`
    /// (`EditDiff(IEnumerable<T>, IEqualityComparer<T>? = null)`).
    fn edit_diff(&self, all_items: impl IntoIterator<Item = T>) {
        edit_diff(self, all_items.into_iter().collect(), &|a: &T, b: &T| a == b);
    }

    /// [`edit_diff`](Self::edit_diff) with the equality of `equality_comparer`
    /// (`EditDiff(IEnumerable<T>, IEqualityComparer<T>)`).
    fn edit_diff_with_comparer(&self, all_items: impl IntoIterator<Item = T>, equality_comparer: impl Fn(&T, &T) -> bool) {
        edit_diff(self, all_items.into_iter().collect(), &equality_comparer);
    }

    /// Inserts `item` at `index` (`Insert`).
    fn insert(&self, index: usize, item: T) {
        let mut item = Some(item);
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(item) = item.take() {
                IList::insert(list, index, item);
            }
        });
    }

    /// Inserts the items at `index` (`InsertRange`, `ListEx.AddRange(items, index)`).
    fn insert_range(&self, items: impl IntoIterator<Item = T>, index: usize) {
        let mut items = Some(items.into_iter().collect::<Vec<T>>());
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(items) = items.take() {
                list.insert_range(items, index);
            }
        });
    }

    /// Moves the item at `original` to `destination` (`Move`; `move` is a keyword of Rust).
    fn move_item(&self, original: usize, destination: usize) {
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| list.move_index(original, destination));
    }

    /// Removes the first item equal to `item`; whether there was one (`Remove`).
    fn remove(&self, item: &T) -> bool {
        let mut removed = false;
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| removed = IList::remove(list, item));
        removed
    }

    /// Removes the item at `index` (`RemoveAt`).
    fn remove_at(&self, index: usize) {
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| IList::remove_at(list, index));
    }

    /// Removes the items (`RemoveMany`).
    fn remove_many(&self, items_to_remove: impl IntoIterator<Item = T>) {
        let items: Vec<T> = items_to_remove.into_iter().collect();
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            let list: &mut dyn IList<T> = list;
            remove_many(list, &items);
        });
    }

    /// Removes `count` items from `index` (`RemoveRange`).
    fn remove_range(&self, index: usize, count: usize) {
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| IList::remove_range(list, index, count));
    }

    /// Replaces `original` with `destination` (`Replace`).
    ///
    /// # Panics
    /// When `original` is not in the list (`ArgumentException`).
    fn replace(&self, original: &T, destination: T) {
        let mut destination = Some(destination);
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(destination) = destination.take() {
                let list: &mut dyn IList<T> = list;
                replace(list, original, destination);
            }
        });
    }

    /// Replaces the item at `index` with `item` (`ReplaceAt`).
    fn replace_at(&self, index: usize, item: T) {
        let mut item = Some(item);
        self.edit_dyn(&mut |list: &mut dyn IExtendedList<T>| {
            if let Some(item) = item.take() {
                IList::set(list, index, item);
            }
        });
    }
}

impl<T: Clone + PartialEq + 'static, L: ISourceList<T> + ?Sized> SourceListEditConvenienceEx<T> for L {}
