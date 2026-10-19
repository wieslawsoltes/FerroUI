//! `ChangeAwareList<T>` and `IExtendedList<T>` (`List/ChangeAwareList.cs`, `List/IExtendedList.cs`).

use super::{Change, ChangeSet, ListChangeReason};
use crate::list::internal::unified;

/// A list with the range operations of DynamicData (`IExtendedList<T>`, an `IList<T>`).
///
/// `AddRange` and `RemoveRange` are [`IList::add_range`](super::IList::add_range) and
/// [`IList::remove_range`](super::IList::remove_range), which a list with ranges overrides.
pub trait IExtendedList<T: Clone + PartialEq>: super::IList<T> {
    /// Inserts the items at `index` (`InsertRange`).
    fn insert_range(&mut self, items: Vec<T>, index: usize);
    /// Moves the item at `original` to `destination` (`Move(int, int)`).
    fn move_index(&mut self, original: usize, destination: usize);
}

/// What a list that tracks more than its changes does at the points upstream's subclasses
/// override (`ChangeAwareListWithRefCounts`).
pub(crate) trait ChangeAwareListHooks<T> {
    fn on_insert_item(&mut self, _item: &T) {}
    fn on_insert_items(&mut self, _items: &[T]) {}
    fn on_remove_item(&mut self, _item: &T) {}
    fn on_remove_items(&mut self, _items: &[T]) {}
    fn on_set_item(&mut self, _new_item: &T, _old_item: &T) {}
    fn on_clear(&mut self) {}
    /// `Some` to answer `Contains` instead of a search of the list.
    fn contains(&self, _item: &T) -> Option<bool> {
        None
    }
}

/// A list that records its changes until they are captured (`ChangeAwareList<T>`), batching
/// consecutive insertions and removals into ranges as upstream does.
pub struct ChangeAwareList<T> {
    inner: Vec<T>,
    changes: Vec<Change<T>>,
    hooks: Option<Box<dyn ChangeAwareListHooks<T>>>,
}

impl<T: Clone + PartialEq> Default for ChangeAwareList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq> ChangeAwareList<T> {
    /// An empty list (`ChangeAwareList(int capacity = -1)`).
    pub fn new() -> Self {
        Self { inner: Vec::new(), changes: Vec::new(), hooks: None }
    }

    /// An empty list with room for `capacity` items.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { inner: Vec::with_capacity(capacity), changes: Vec::new(), hooks: None }
    }

    /// A list of the items, whose first change adds them (`ChangeAwareList(IEnumerable<T>)`).
    pub fn from_items(items: Vec<T>) -> Self {
        let mut changes = Vec::new();
        if !items.is_empty() {
            changes.push(Change::range_change(ListChangeReason::AddRange, items.clone(), None));
        }
        Self { inner: items, changes, hooks: None }
    }

    /// A copy of `list`, with its changes or without (`ChangeAwareList(ChangeAwareList<T>, bool)`).
    pub fn from_list(list: &ChangeAwareList<T>, copy_changes: bool) -> Self {
        Self { inner: list.inner.clone(), changes: if copy_changes { list.changes.clone() } else { Vec::new() }, hooks: None }
    }

    pub(crate) fn with_hooks(hooks: Box<dyn ChangeAwareListHooks<T>>) -> Self {
        Self { inner: Vec::new(), changes: Vec::new(), hooks: Some(hooks) }
    }

    /// The number of items (`Count`).
    pub fn count(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// The items.
    pub fn items(&self) -> &[T] {
        &self.inner
    }

    /// The item at `index` (the indexer's getter).
    pub fn get(&self, index: usize) -> &T {
        &self.inner[index]
    }

    /// Replaces the item at `index` (the indexer's setter, `SetItem`).
    pub fn set(&mut self, index: usize, item: T) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        let previous = self.inner[index].clone();
        self.changes.push(Change::with_previous(ListChangeReason::Replace, item.clone(), Some(previous.clone()), Some(index), Some(index)));
        self.inner[index] = item.clone();
        if let Some(hooks) = &mut self.hooks {
            hooks.on_set_item(&item, &previous);
        }
    }

    /// Adds an item at the end (`Add`).
    pub fn add(&mut self, item: T) {
        let index = self.inner.len();
        self.insert_item(index, item);
    }

    /// The changes since the last capture, and a clean slate (`CaptureChanges`): removals that
    /// emptied the list are reported as one `Clear`.
    pub fn capture_changes(&mut self) -> ChangeSet<T> {
        if self.changes.is_empty() {
            return ChangeSet::empty();
        }
        let mut changes = ChangeSet::from(std::mem::take(&mut self.changes));
        // We can infer this is a clear.
        if self.inner.is_empty() && changes.removes() == changes.total_changes() && changes.total_changes() > 1 {
            let removed: Vec<T> = unified(&changes).into_iter().map(|change| change.current).collect();
            changes = ChangeSet::from(vec![Change::range_change(ListChangeReason::Clear, removed, None)]);
        }
        changes
    }

    /// Removes every item (`Clear`).
    pub fn clear(&mut self) {
        if let Some(hooks) = &mut self.hooks {
            hooks.on_clear();
        }
        if self.inner.is_empty() {
            return;
        }
        let removed = std::mem::take(&mut self.inner);
        self.changes.push(Change::range_change(ListChangeReason::Clear, removed, None));
    }

    /// Whether the list has `item` (`Contains`).
    pub fn contains(&self, item: &T) -> bool {
        if let Some(answer) = self.hooks.as_ref().and_then(|hooks| hooks.contains(item)) {
            return answer;
        }
        self.inner.contains(item)
    }

    /// The index of `item` (`IndexOf`).
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.inner.iter().position(|candidate| candidate == item)
    }

    /// The index of `item` by `comparer` (`IndexOf(T, IEqualityComparer<T>)`).
    pub fn index_of_with(&self, item: &T, comparer: &dyn Fn(&T, &T) -> bool) -> Option<usize> {
        self.inner.iter().position(|candidate| comparer(candidate, item))
    }

    /// Inserts an item (`Insert`).
    ///
    /// # Panics
    /// When `index` is beyond the end (`ArgumentException`).
    pub fn insert(&mut self, index: usize, item: T) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        self.insert_item(index, item);
    }

    /// Moves `item` to `destination` (`Move(T, int)`).
    pub fn move_item(&mut self, item: &T, destination: usize) {
        assert!(destination <= self.inner.len(), "destination cannot be greater than the size of the collection");
        let index = self.index_of(item).expect("the item to move is in the list");
        self.move_index(index, destination);
    }

    /// Replaces the item at `index` with `item` and reports a refresh (`Refresh(T, int)`).
    pub fn refresh_with(&mut self, item: T, index: usize) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        let previous = std::mem::replace(&mut self.inner[index], item.clone());
        self.changes.push(Change::with_previous(ListChangeReason::Refresh, item, Some(previous), Some(index), None));
    }

    /// Reports a refresh of `item`; false when it is not in the list (`Refresh(T)`).
    pub fn refresh(&mut self, item: &T) -> bool {
        let Some(index) = self.index_of(item) else { return false };
        self.changes.push(Change::new(ListChangeReason::Refresh, item.clone(), Some(index)));
        true
    }

    /// Reports a refresh of the item at `index` (`RefreshAt`).
    pub fn refresh_at(&mut self, index: usize) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        self.changes.push(Change::new(ListChangeReason::Refresh, self.inner[index].clone(), Some(index)));
    }

    /// Removes `item`; false when it is not in the list (`Remove(T)`).
    pub fn remove(&mut self, item: &T) -> bool {
        let Some(index) = self.index_of(item) else { return false };
        let item = self.inner[index].clone();
        self.remove_item(index, item);
        true
    }

    /// Removes the item at `index` (`RemoveAt`).
    pub fn remove_at(&mut self, index: usize) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        let item = self.inner[index].clone();
        self.remove_item(index, item);
    }

    /// Inserts with the batching of consecutive insertions into a range (`InsertItem`).
    fn insert_item(&mut self, index: usize, item: T) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        if let Some(hooks) = &mut self.hooks {
            hooks.on_insert_item(&item);
        }
        // Attempt to batch updates, as lists love to deal with ranges.
        let last_reason = self.changes.last().map(Change::reason);
        match last_reason {
            Some(ListChangeReason::Add) => {
                let first_of_batch = self.changes.len() - 1;
                let previous = self.changes[first_of_batch].item().clone();
                let previous_index = previous.current_index();
                if Some(index) == previous_index {
                    self.changes[first_of_batch] = Change::range_change(ListChangeReason::AddRange, vec![item.clone(), previous.current().clone()], Some(index));
                } else if previous_index.is_some_and(|previous_index| index == previous_index + 1) {
                    self.changes[first_of_batch] = Change::range_change(ListChangeReason::AddRange, vec![previous.current().clone(), item.clone()], previous_index);
                } else {
                    self.changes.push(Change::new(ListChangeReason::Add, item.clone(), Some(index)));
                }
            }
            Some(ListChangeReason::AddRange) => {
                let last = self.changes.last_mut().expect("there is a last change");
                let range = last.range_mut();
                // Upstream's index of a range without one is -1.
                let range_index = range.index().map_or(-1, |index| index as isize);
                let minimum = (range_index - 1).max(0);
                let maximum = range_index + range.count() as isize;
                let position = index as isize;
                if position >= minimum && position <= maximum {
                    let insert_position = (position - range_index).clamp(0, range.count() as isize) as usize;
                    range.insert(insert_position, item.clone());
                    if position < range_index {
                        range.set_starting_index(Some(index));
                    }
                } else {
                    self.changes.push(Change::new(ListChangeReason::Add, item.clone(), Some(index)));
                }
            }
            // The first insertion: no range can be inferred.
            _ => self.changes.push(Change::new(ListChangeReason::Add, item.clone(), Some(index))),
        }
        self.inner.insert(index, item);
    }

    /// Removes with the batching of consecutive removals into a range (`RemoveItem`).
    fn remove_item(&mut self, index: usize, item: T) {
        assert!(index <= self.inner.len(), "index cannot be greater than the size of the collection");
        if let Some(hooks) = &mut self.hooks {
            hooks.on_remove_item(&item);
        }
        let last_reason = self.changes.last().map(Change::reason);
        match last_reason {
            Some(ListChangeReason::Remove) => {
                let first_of_batch = self.changes.len() - 1;
                let previous = self.changes[first_of_batch].item().clone();
                let previous_index = previous.current_index();
                if Some(index) == previous_index {
                    self.changes[first_of_batch] = Change::range_change(ListChangeReason::RemoveRange, vec![previous.current().clone(), item.clone()], Some(index));
                } else if previous_index.is_some_and(|previous_index| index + 1 == previous_index) {
                    self.changes[first_of_batch] = Change::range_change(ListChangeReason::RemoveRange, vec![item.clone(), previous.current().clone()], Some(index));
                } else {
                    self.changes.push(Change::new(ListChangeReason::Remove, item.clone(), Some(index)));
                }
            }
            Some(ListChangeReason::RemoveRange) => {
                let range_index = self.changes.last().expect("there is a last change").range().index();
                if range_index == Some(index) {
                    // Removed in order.
                    self.changes.last_mut().expect("there is a last change").range_mut().add(item.clone());
                } else if range_index == Some(index + 1) {
                    // Removed in reverse order.
                    let range = self.changes.last_mut().expect("there is a last change").range_mut();
                    range.insert(0, item.clone());
                    range.set_starting_index(Some(index));
                } else {
                    // Including the case upstream checks apart (`range.Index == index - 1`).
                    self.changes.push(Change::new(ListChangeReason::Remove, item.clone(), Some(index)));
                }
            }
            // The first removal: no range can be inferred.
            _ => self.changes.push(Change::new(ListChangeReason::Remove, item.clone(), Some(index))),
        }
        self.inner.remove(index);
    }
}

impl<T: Clone + PartialEq> ChangeAwareList<T> {
    /// Adds the items at the end (`AddRange`).
    pub fn add_range(&mut self, items: Vec<T>) {
        if items.is_empty() {
            return;
        }
        let index = self.inner.len();
        self.changes.push(Change::range_change(ListChangeReason::AddRange, items.clone(), Some(index)));
        if let Some(hooks) = &mut self.hooks {
            hooks.on_insert_items(&items);
        }
        self.inner.extend(items);
    }

    /// Removes `count` items from `index` (`RemoveRange`).
    pub fn remove_range(&mut self, index: usize, count: usize) {
        assert!(count <= self.inner.len() && index <= self.inner.len() - count, "the range is beyond the end of the collection");
        let removed: Vec<T> = self.inner[index..index + count].to_vec();
        if removed.is_empty() {
            return;
        }
        self.changes.push(Change::range_change(ListChangeReason::RemoveRange, removed.clone(), Some(index)));
        self.inner.drain(index..index + count);
        if let Some(hooks) = &mut self.hooks {
            hooks.on_remove_items(&removed);
        }
    }
}

impl<T: Clone + PartialEq> IExtendedList<T> for ChangeAwareList<T> {
    fn insert_range(&mut self, items: Vec<T>, index: usize) {
        if items.is_empty() {
            return;
        }
        self.changes.push(Change::range_change(ListChangeReason::AddRange, items.clone(), Some(index)));
        self.inner.splice(index..index, items.iter().cloned());
        if let Some(hooks) = &mut self.hooks {
            hooks.on_insert_items(&items);
        }
    }

    fn move_index(&mut self, original: usize, destination: usize) {
        assert!(original <= self.inner.len(), "original cannot be greater than the size of the collection");
        assert!(destination <= self.inner.len(), "destination cannot be greater than the size of the collection");
        let item = self.inner.remove(original);
        self.inner.insert(destination, item.clone());
        self.changes.push(Change::moved(item, destination, original));
    }

}
