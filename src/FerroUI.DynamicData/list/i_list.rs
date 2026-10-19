//! `IList<T>` as DynamicData uses it, and `ListEx` (`List/ListEx.cs`): applying the changes of a
//! list to another list.
//!
//! Upstream's `ListEx` takes any `IList<T>` and branches on its type: `List<T>` and
//! `IExtendedList<T>` insert and remove ranges at once, `ObservableCollection<T>` moves items
//! itself, `ChangeAwareList<T>` reports refreshes. Here those are provided methods of [`IList`]
//! with upstream's general form, which those types override.

use super::{Change, ChangeAwareList, ChangeSet, IExtendedList, ListChangeReason};

/// A list that the changes of a list can be applied to (`System.Collections.Generic.IList<T>`,
/// with the forms of `ListEx` as provided methods).
pub trait IList<T: Clone + PartialEq> {
    /// The number of items (`Count`).
    fn count(&self) -> usize;
    /// The item at `index` (the indexer's getter).
    fn get(&self, index: usize) -> T;
    /// Replaces the item at `index` (the indexer's setter).
    fn set(&mut self, index: usize, item: T);
    /// Inserts an item (`Insert`).
    fn insert(&mut self, index: usize, item: T);
    /// Removes the item at `index` (`RemoveAt`).
    fn remove_at(&mut self, index: usize);
    /// Removes every item (`Clear`).
    fn clear(&mut self);

    /// Adds an item at the end (`Add`).
    fn add(&mut self, item: T) {
        let index = self.count();
        self.insert(index, item);
    }

    /// The index of `item` (`IndexOf`).
    fn index_of(&self, item: &T) -> Option<usize> {
        (0..self.count()).find(|&index| &self.get(index) == item)
    }

    /// The index of `item` by `comparer` (`ListEx.IndexOf(IEnumerable, T, IEqualityComparer)`).
    fn index_of_with(&self, item: &T, comparer: &dyn Fn(&T, &T) -> bool) -> Option<usize> {
        (0..self.count()).find(|&index| comparer(item, &self.get(index)))
    }

    /// Removes `item`; false when it is not in the list (`Remove`).
    fn remove(&mut self, item: &T) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Adds the items at the end (`ListEx.AddRange`): one by one unless the list adds ranges.
    fn add_range(&mut self, items: Vec<T>) {
        for item in items {
            self.add(item);
        }
    }

    /// Inserts the items at `index`, or adds them without an index (`ListEx.AddOrInsertRange`): in
    /// reverse order at the index unless the list inserts ranges, as upstream does.
    fn add_or_insert_range(&mut self, items: Vec<T>, index: Option<usize>) {
        match index {
            Some(index) => {
                for item in items.into_iter().rev() {
                    self.insert(index, item);
                }
            }
            None => self.add_range(items),
        }
    }

    /// Removes `count` items from `index` (`ListEx.RemoveRange`).
    ///
    /// # Panics
    /// For a list without ranges (`NotSupportedException`).
    fn remove_range(&mut self, _index: usize, _count: usize) {
        panic!("Cannot remove range from this list");
    }

    /// Whether the list removes ranges at once (`List<T>` or `IExtendedList<T>`).
    fn supports_ranges(&self) -> bool {
        false
    }

    /// Moves the item at `original` to `destination` (the `Moved` case of `ListEx.Clone`).
    fn move_item(&mut self, original: usize, destination: usize) {
        let item = self.get(original);
        self.remove_at(original);
        self.insert(destination, item);
    }

    /// Reports a refresh of the item at `index` (the `Refresh` case of `ListEx.Clone`): the item is
    /// removed and inserted again unless the list reports refreshes.
    fn refresh_at(&mut self, index: usize, item: T) {
        self.remove_at(index);
        self.insert(index, item);
    }
}

impl<T: Clone + PartialEq> IList<T> for Vec<T> {
    fn count(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> T {
        self[index].clone()
    }

    fn set(&mut self, index: usize, item: T) {
        self[index] = item;
    }

    fn insert(&mut self, index: usize, item: T) {
        Vec::insert(self, index, item);
    }

    fn remove_at(&mut self, index: usize) {
        Vec::remove(self, index);
    }

    fn clear(&mut self) {
        Vec::clear(self);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        self.iter().position(|candidate| candidate == item)
    }

    fn add_range(&mut self, items: Vec<T>) {
        self.extend(items);
    }

    fn add_or_insert_range(&mut self, items: Vec<T>, index: Option<usize>) {
        match index {
            Some(index) => {
                self.splice(index..index, items);
            }
            None => self.extend(items),
        }
    }

    fn remove_range(&mut self, index: usize, count: usize) {
        self.drain(index..index + count);
    }

    fn supports_ranges(&self) -> bool {
        true
    }
}

impl<T: Clone + PartialEq> IList<T> for ChangeAwareList<T> {
    fn count(&self) -> usize {
        ChangeAwareList::count(self)
    }

    fn get(&self, index: usize) -> T {
        ChangeAwareList::get(self, index).clone()
    }

    fn set(&mut self, index: usize, item: T) {
        ChangeAwareList::set(self, index, item);
    }

    fn insert(&mut self, index: usize, item: T) {
        ChangeAwareList::insert(self, index, item);
    }

    fn remove_at(&mut self, index: usize) {
        ChangeAwareList::remove_at(self, index);
    }

    fn clear(&mut self) {
        ChangeAwareList::clear(self);
    }

    fn add(&mut self, item: T) {
        ChangeAwareList::add(self, item);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        ChangeAwareList::index_of(self, item)
    }

    fn remove(&mut self, item: &T) -> bool {
        ChangeAwareList::remove(self, item)
    }

    fn add_range(&mut self, items: Vec<T>) {
        ChangeAwareList::add_range(self, items);
    }

    fn add_or_insert_range(&mut self, items: Vec<T>, index: Option<usize>) {
        match index {
            Some(index) => IExtendedList::insert_range(self, items, index),
            None => ChangeAwareList::add_range(self, items),
        }
    }

    fn remove_range(&mut self, index: usize, count: usize) {
        ChangeAwareList::remove_range(self, index, count);
    }

    fn supports_ranges(&self) -> bool {
        true
    }

    fn move_item(&mut self, original: usize, destination: usize) {
        IExtendedList::move_index(self, original, destination);
    }

    fn refresh_at(&mut self, index: usize, _item: T) {
        ChangeAwareList::refresh_at(self, index);
    }
}

/// Applies the changes to `target` (`ListEx.Clone(IList<T>, IChangeSet<T>)`).
pub fn clone_changes<T: Clone + PartialEq>(target: &mut dyn IList<T>, changes: &ChangeSet<T>) {
    for change in changes {
        clone_change(target, change, &|a, b| a == b);
    }
}

/// Applies the changes to `target`, comparing items with `comparer`
/// (`ListEx.Clone(IList<T>, IEnumerable<Change<T>>, IEqualityComparer<T>)`).
pub fn clone_changes_with<T: Clone + PartialEq>(target: &mut dyn IList<T>, changes: &[Change<T>], comparer: &dyn Fn(&T, &T) -> bool) {
    for change in changes {
        clone_change(target, change, comparer);
    }
}

/// Removes the items of the change, or clears the list when they are all of it
/// (`ListEx.ClearOrRemoveMany`).
pub fn clear_or_remove_many<T: Clone + PartialEq>(target: &mut dyn IList<T>, change: &Change<T>) {
    if target.count() == change.range().count() {
        target.clear();
    } else {
        remove_many(target, change.range().items());
    }
}

/// Removes the items (`ListEx.RemoveMany`): by their indexes from the last, unless an item is
/// there more than once, when each is removed by a search.
pub fn remove_many<T: Clone + PartialEq>(target: &mut dyn IList<T>, items: &[T]) {
    let mut indexes: Vec<(usize, &T)> = items.iter().filter_map(|item| target.index_of(item).map(|index| (index, item))).collect();
    indexes.sort_by(|a, b| b.0.cmp(&a.0));
    let has_duplicates = indexes.iter().enumerate().any(|(i, (_, item))| indexes[..i].iter().any(|(_, other)| other == item));
    if has_duplicates {
        for item in items {
            target.remove(item);
        }
    } else {
        for (index, _) in indexes {
            target.remove_at(index);
        }
    }
}

/// Replaces `original` with `replace_with`, or adds it when `original` is not in the list
/// (`ListEx.ReplaceOrAdd`).
pub fn replace_or_add<T: Clone + PartialEq>(target: &mut dyn IList<T>, original: &T, replace_with: T) {
    match target.index_of(original) {
        Some(index) => target.set(index, replace_with),
        None => target.add(replace_with),
    }
}

/// Replaces `original` with `replace_with` (`ListEx.Replace`).
///
/// # Panics
/// When `original` is not in the list (`ArgumentException`).
pub fn replace<T: Clone + PartialEq>(target: &mut dyn IList<T>, original: &T, replace_with: T) {
    let index = target.index_of(original).expect("Cannot find index of original item. Either it does not exist in the list or the hashcode has mutated");
    target.set(index, replace_with);
}

/// The index of `value` by a comparison, or the bitwise complement of where it belongs
/// (`ListEx.BinarySearch`): `Err(index)` for the latter.
pub fn binary_search_by<T: Clone + PartialEq, S>(list: &dyn IList<T>, value: &S, comparer: &dyn Fn(&S, &T) -> std::cmp::Ordering) -> Result<usize, usize> {
    let (mut lower, mut upper) = (0isize, list.count() as isize - 1);
    while lower <= upper {
        let middle = lower + (upper - lower) / 2;
        match comparer(value, &list.get(middle as usize)) {
            std::cmp::Ordering::Less => upper = middle - 1,
            std::cmp::Ordering::Greater => lower = middle + 1,
            std::cmp::Ordering::Equal => return Ok(middle as usize),
        }
    }
    Err(lower as usize)
}

/// Whether a move touches the range from `start_index` to `end_index` (`MovedWithinRange`).
pub(crate) fn moved_within_range<T>(change: &Change<T>, start_index: usize, end_index: usize) -> bool {
    if change.reason() != ListChangeReason::Moved {
        return false;
    }
    let item = change.item();
    let within = |index: Option<usize>| index.is_some_and(|index| index >= start_index && index <= end_index);
    within(item.current_index()) || within(item.previous_index())
}

/// One change applied to `target` (the private `Clone(IList<T>, Change<T>, ..)`).
fn clone_change<T: Clone + PartialEq>(target: &mut dyn IList<T>, change: &Change<T>, comparer: &dyn Fn(&T, &T) -> bool) {
    match change.reason() {
        ListChangeReason::Add => {
            let item = change.item();
            match item.current_index() {
                Some(index) => target.insert(index, item.current().clone()),
                None => target.add(item.current().clone()),
            }
        }
        ListChangeReason::AddRange => target.add_or_insert_range(change.range().items().to_vec(), change.range().index()),
        ListChangeReason::Clear => clear_or_remove_many(target, change),
        ListChangeReason::Replace => {
            let item = change.item();
            if item.current_index().is_some() && item.current_index() == item.previous_index() {
                target.set(item.current_index().expect("checked"), item.current().clone());
            } else {
                match item.previous_index() {
                    None => {
                        target.remove(item.previous().expect("a replacement has a previous item"));
                    }
                    // Is this best, or a replacement and a move?
                    Some(index) => target.remove_at(index),
                }
                match item.current_index() {
                    None => target.add(item.current().clone()),
                    Some(index) => target.insert(index, item.current().clone()),
                }
            }
        }
        ListChangeReason::Refresh => {
            let item = change.item();
            let index = item.current_index().expect("a refresh has an index");
            target.refresh_at(index, item.current().clone());
        }
        ListChangeReason::Remove => {
            let item = change.item();
            match item.current_index() {
                Some(index) => target.remove_at(index),
                None => {
                    if let Some(index) = target.index_of_with(item.current(), comparer) {
                        target.remove_at(index);
                    }
                }
            }
        }
        ListChangeReason::RemoveRange => {
            // A range without an index (one that `WhereReasonsAre` stripped) is removed item by
            // item.
            match change.range().index() {
                Some(index) if target.supports_ranges() => target.remove_range(index, change.range().count()),
                _ => remove_many(target, change.range().items()),
            }
        }
        ListChangeReason::Moved => {
            let item = change.item();
            let current = item.current_index().expect("Cannot move as an index was not specified");
            let previous = item.previous_index().expect("Cannot move as an index was not specified");
            target.move_item(previous, current);
        }
    }
}
