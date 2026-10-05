use crate::utilities::HandlerList;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Describes the action that caused a collection changed notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotifyCollectionChangedAction {
    Add,
    Remove,
    Replace,
    Move,
    Reset,
}

/// Describes a change to a collection.
///
/// Indexes are `-1` when not applicable to the action, as in the notification
/// model this mirrors.
pub struct NotifyCollectionChangedEventArgs<'a, T> {
    pub action: NotifyCollectionChangedAction,
    pub new_items: &'a [T],
    pub old_items: &'a [T],
    pub new_starting_index: i32,
    pub old_starting_index: i32,
}

impl<T: Clone> NotifyCollectionChangedEventArgs<'_, T> {
    /// The change as an owned, shareable value: what untyped code (an
    /// event handler wired up by markup) is given, since the borrowed form
    /// cannot be held in an untyped value.
    pub fn share(&self) -> SharedNotifyCollectionChangedEventArgs<T> {
        SharedNotifyCollectionChangedEventArgs(Rc::new(SharedNotifyCollectionChangedData {
            action: self.action,
            new_items: self.new_items.to_vec(),
            old_items: self.old_items.to_vec(),
            new_starting_index: self.new_starting_index,
            old_starting_index: self.old_starting_index,
        }))
    }
}

struct SharedNotifyCollectionChangedData<T> {
    action: NotifyCollectionChangedAction,
    new_items: Vec<T>,
    old_items: Vec<T>,
    new_starting_index: i32,
    old_starting_index: i32,
}

/// The owned form of [`NotifyCollectionChangedEventArgs`]: a shared handle
/// to the description of a change. Clones refer to the same description and
/// handles compare by identity, as the event args object this mirrors.
pub struct SharedNotifyCollectionChangedEventArgs<T>(Rc<SharedNotifyCollectionChangedData<T>>);

impl<T> Clone for SharedNotifyCollectionChangedEventArgs<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for SharedNotifyCollectionChangedEventArgs<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> SharedNotifyCollectionChangedEventArgs<T> {
    /// The action that caused the notification.
    pub fn action(&self) -> NotifyCollectionChangedAction {
        self.0.action
    }

    /// The items involved in the change that are new.
    pub fn new_items(&self) -> &[T] {
        &self.0.new_items
    }

    /// The items affected by a replace, remove or move.
    pub fn old_items(&self) -> &[T] {
        &self.0.old_items
    }

    /// The index at which the change occurred; `-1` when not applicable.
    pub fn new_starting_index(&self) -> i32 {
        self.0.new_starting_index
    }

    /// The index at which a move, remove or replace occurred; `-1` when not
    /// applicable.
    pub fn old_starting_index(&self) -> i32 {
        self.0.old_starting_index
    }

    /// The change in the borrowed form handlers take.
    pub fn as_args(&self) -> NotifyCollectionChangedEventArgs<'_, T> {
        NotifyCollectionChangedEventArgs {
            action: self.0.action,
            new_items: &self.0.new_items,
            old_items: &self.0.old_items,
            new_starting_index: self.0.new_starting_index,
            old_starting_index: self.0.old_starting_index,
        }
    }
}

/// Describes the action notified on a clear of a [`FerroList`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResetBehavior {
    /// Clearing the list notifies a with a [`NotifyCollectionChangedAction::Reset`].
    #[default]
    Reset,
    /// Clearing the list notifies a with a [`NotifyCollectionChangedAction::Remove`].
    Remove,
}

/// The signature of a collection changed handler.
pub type CollectionChangedHandler<T> = dyn for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, T>);

/// A notifying list.
///
/// Enumeration works on a snapshot, so the list may be modified from within
/// an enumeration or a change notification. The snapshot shares storage with
/// the list and is only copied if the list is modified while it is alive.
///
/// The list is a reference object: the value is a shared handle, clones
/// refer to the same list and handles compare by identity.
pub struct FerroList<T>(Rc<FerroListData<T>>);

struct FerroListData<T> {
    items: RefCell<Rc<Vec<T>>>,
    collection_changed: HandlerList<CollectionChangedHandler<T>>,
    validator: RefCell<Option<Rc<dyn Fn(&T)>>>,
    reset_behavior: Cell<ResetBehavior>,
}

impl<T> Clone for FerroList<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for FerroList<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> FerroList<T> {
    /// Whether two handles refer to the same list.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> std::fmt::Debug for FerroList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FerroList({})", self.0.items.borrow().len())
    }
}

impl<T: Clone> Default for FerroList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> FerroList<T> {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self(Rc::new(FerroListData {
            items: RefCell::new(Rc::new(Vec::new())),
            collection_changed: HandlerList::new(),
            validator: RefCell::new(None),
            reset_behavior: Cell::new(ResetBehavior::Reset),
        }))
    }

    /// Creates a list with initial items.
    pub fn from_items(items: impl IntoIterator<Item = T>) -> Self {
        let list = Self::new();
        *list.0.items.borrow_mut() = Rc::new(items.into_iter().collect());
        list
    }

    /// Subscribes to changes. Returns a token for
    /// [`remove_collection_changed`](Self::remove_collection_changed).
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        self.0.collection_changed.add(handler)
    }

    /// Unsubscribes a handler.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.collection_changed.remove(token)
    }

    /// Whether anything is subscribed to changes.
    pub fn has_collection_changed_subscribers(&self) -> bool {
        !self.0.collection_changed.is_empty()
    }

    /// The number of items in the list.
    #[inline]
    pub fn count(&self) -> usize {
        self.0.items.borrow().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.items.borrow().is_empty()
    }

    /// The number of items the list can hold before its storage grows.
    pub fn capacity(&self) -> usize {
        self.0.items.borrow().capacity()
    }

    /// Sets the number of items the list can hold before its storage grows.
    ///
    /// # Panics
    /// Panics if `capacity` is less than the number of items, as the
    /// managed original throws.
    pub fn set_capacity(&self, capacity: usize) {
        let mut items = self.0.items.borrow_mut();
        assert!(capacity >= items.len(), "capacity was less than the current size.");
        if capacity == items.capacity() {
            return;
        }
        // The storage of a snapshot that is still alive is left to the snapshot.
        let mut resized = Vec::with_capacity(capacity);
        resized.extend(items.iter().cloned());
        *items = Rc::new(resized);
    }

    /// The reset behavior of the list.
    pub fn reset_behavior(&self) -> ResetBehavior {
        self.0.reset_behavior.get()
    }

    pub fn set_reset_behavior(&self, value: ResetBehavior) {
        self.0.reset_behavior.set(value)
    }

    /// Sets a validation routine that is invoked for every item before it is
    /// added to the list; it rejects an item by panicking.
    pub fn set_validator(&self, validator: Option<Rc<dyn Fn(&T)>>) {
        *self.0.validator.borrow_mut() = validator;
    }

    /// The item at `index`. Panics if out of range.
    #[inline]
    pub fn get(&self, index: usize) -> T {
        self.0.items.borrow()[index].clone()
    }

    /// The item at `index`, if in range.
    #[inline]
    pub fn try_get(&self, index: usize) -> Option<T> {
        self.0.items.borrow().get(index).cloned()
    }

    /// Replaces the item at `index`.
    pub fn set(&self, index: usize, value: T)
    where
        T: PartialEq,
    {
        self.validate(&value);
        let old = self.get(index);
        if old != value {
            self.modify(|items| items[index] = value.clone());
            self.notify(NotifyCollectionChangedEventArgs {
                action: NotifyCollectionChangedAction::Replace,
                new_items: std::slice::from_ref(&value),
                old_items: std::slice::from_ref(&old),
                new_starting_index: index as i32,
                old_starting_index: index as i32,
            });
        }
    }

    /// A snapshot of the items. It stays valid and unchanged if the list is
    /// modified.
    #[inline]
    pub fn snapshot(&self) -> Rc<Vec<T>> {
        self.0.items.borrow().clone()
    }

    /// Copies the items into a vector.
    pub fn to_vec(&self) -> Vec<T> {
        self.0.items.borrow().as_ref().clone()
    }

    /// Calls `f` for each item of a snapshot of the list.
    pub fn for_each(&self, mut f: impl FnMut(&T)) {
        if self.is_empty() {
            return;
        }
        for item in self.snapshot().iter() {
            f(item);
        }
    }

    /// Adds an item to the collection.
    pub fn add(&self, item: T) {
        self.validate(&item);
        let index = self.count();
        self.modify(|items| items.push(item.clone()));
        self.notify_add(std::slice::from_ref(&item), index);
    }

    /// Adds multiple items to the collection.
    pub fn add_range(&self, items: impl IntoIterator<Item = T>) {
        self.insert_range(self.count(), items)
    }

    /// Inserts an item at the specified index.
    pub fn insert(&self, index: usize, item: T) {
        self.validate(&item);
        self.modify(|items| items.insert(index, item.clone()));
        self.notify_add(std::slice::from_ref(&item), index);
    }

    /// Inserts multiple items at the specified index.
    pub fn insert_range(&self, index: usize, items: impl IntoIterator<Item = T>) {
        let added: Vec<T> = items.into_iter().collect();
        if added.is_empty() {
            return;
        }
        for item in &added {
            self.validate(item);
        }
        self.modify(|list| {
            list.splice(index..index, added.iter().cloned());
        });
        self.notify_add(&added, index);
    }

    /// Moves an item to a new index.
    pub fn move_item(&self, old_index: usize, new_index: usize) {
        if old_index == new_index {
            return;
        }
        let item = self.get(old_index);
        self.modify(|items| {
            let item = items.remove(old_index);
            items.insert(new_index, item);
        });
        self.notify(NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Move,
            new_items: std::slice::from_ref(&item),
            old_items: std::slice::from_ref(&item),
            new_starting_index: new_index as i32,
            old_starting_index: old_index as i32,
        });
    }

    /// Moves multiple items to a new index.
    pub fn move_range(&self, old_index: usize, count: usize, new_index: usize) {
        if old_index == new_index || count == 0 {
            return;
        }
        let moved: Vec<T> = self.0.items.borrow()[old_index..old_index + count].to_vec();
        let mut insert_at = new_index;
        if new_index > old_index {
            insert_at -= count - 1;
        }
        self.modify(|items| {
            items.drain(old_index..old_index + count);
            items.splice(insert_at..insert_at, moved.iter().cloned());
        });
        self.notify(NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Move,
            new_items: &moved,
            old_items: &moved,
            new_starting_index: new_index as i32,
            old_starting_index: old_index as i32,
        });
    }

    /// Removes the item at the specified index.
    pub fn remove_at(&self, index: usize) {
        let item = self.get(index);
        self.modify(|items| {
            items.remove(index);
        });
        self.notify_remove(std::slice::from_ref(&item), index);
    }

    /// Removes a range of elements from the collection.
    pub fn remove_range(&self, index: usize, count: usize) {
        if count == 0 {
            return;
        }
        let removed: Vec<T> = self.0.items.borrow()[index..index + count].to_vec();
        self.modify(|items| {
            items.drain(index..index + count);
        });
        self.notify_remove(&removed, index);
    }

    /// Removes all items from the collection.
    pub fn clear(&self) {
        if self.is_empty() {
            return;
        }
        let old = std::mem::replace(&mut *self.0.items.borrow_mut(), Rc::new(Vec::new()));
        if self.0.collection_changed.is_empty() {
            return;
        }
        match self.0.reset_behavior.get() {
            ResetBehavior::Reset => self.notify(NotifyCollectionChangedEventArgs {
                action: NotifyCollectionChangedAction::Reset,
                new_items: &[],
                old_items: &[],
                new_starting_index: -1,
                old_starting_index: -1,
            }),
            ResetBehavior::Remove => self.notify_remove(&old, 0),
        }
    }

    fn validate(&self, item: &T) {
        let validator = self.0.validator.borrow().clone();
        if let Some(validator) = validator {
            validator(item);
        }
    }

    fn modify(&self, f: impl FnOnce(&mut Vec<T>)) {
        let mut items = self.0.items.borrow_mut();
        f(Rc::make_mut(&mut items));
    }

    fn notify(&self, e: NotifyCollectionChangedEventArgs<'_, T>) {
        if self.0.collection_changed.is_empty() {
            return;
        }
        for (_, handler) in self.0.collection_changed.snapshot().iter() {
            handler(&e);
        }
    }

    fn notify_add(&self, items: &[T], index: usize) {
        self.notify(NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Add,
            new_items: items,
            old_items: &[],
            new_starting_index: index as i32,
            old_starting_index: -1,
        });
    }

    fn notify_remove(&self, items: &[T], index: usize) {
        self.notify(NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Remove,
            new_items: &[],
            old_items: items,
            new_starting_index: -1,
            old_starting_index: index as i32,
        });
    }
}

impl<T: Clone + PartialEq> FerroList<T> {
    /// Tests whether the collection contains an item.
    pub fn contains(&self, item: &T) -> bool {
        self.0.items.borrow().contains(item)
    }

    /// The index of an item in the collection.
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.0.items.borrow().iter().position(|i| i == item)
    }

    /// Removes an item from the collection. Returns true if it was present.
    pub fn remove(&self, item: &T) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Removes multiple items from the collection.
    pub fn remove_all(&self, items: impl IntoIterator<Item = T>) {
        for item in items {
            self.remove(&item);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn capacity_is_read_and_set() {
        let list = super::FerroList::from_items([1, 2, 3]);
        list.set_capacity(16);
        assert_eq!(list.capacity(), 16);
        assert_eq!(list.to_vec(), vec![1, 2, 3]);
        // A snapshot taken before keeps its items.
        let snapshot = list.snapshot();
        list.set_capacity(3);
        assert_eq!(list.capacity(), 3);
        assert_eq!(*snapshot, vec![1, 2, 3]);
    }

    #[test]
    #[should_panic(expected = "capacity was less than the current size.")]
    fn capacity_below_the_count_is_rejected() {
        super::FerroList::from_items([1, 2, 3]).set_capacity(2);
    }

    use super::*;

    fn tracked(list: &FerroList<i32>) -> Rc<RefCell<Vec<(NotifyCollectionChangedAction, Vec<i32>, Vec<i32>, i32, i32)>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, i32>| {
            l.borrow_mut().push((
                e.action,
                e.new_items.to_vec(),
                e.old_items.to_vec(),
                e.new_starting_index,
                e.old_starting_index,
            ));
        }));
        log
    }

    #[test]
    fn items_passed_to_constructor_should_appear_in_list() {
        let target = FerroList::from_items([1, 2, 3]);
        assert_eq!(target.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn adding_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);
        target.add(3);
        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Add, vec![3], vec![], 2, -1)]);
    }

    #[test]
    fn replacing_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);
        target.set(1, 3);
        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Replace, vec![3], vec![2], 1, 1)]);
    }

    #[test]
    fn move_range_should_update_collection() {
        let target = FerroList::from_items([1, 2, 3]);
        target.move_range(0, 2, 2);
        assert_eq!(target.to_vec(), vec![3, 1, 2]);
        let target = FerroList::from_items([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        target.move_range(0, 5, 10 - 1);
        assert_eq!(target.to_vec(), vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn clearing_items_should_raise_reset_or_remove() {
        let target = FerroList::from_items([1, 2, 3]);
        let log = tracked(&target);
        target.clear();
        assert_eq!(log.borrow()[0].0, NotifyCollectionChangedAction::Reset);

        let target = FerroList::from_items([1, 2, 3]);
        target.set_reset_behavior(ResetBehavior::Remove);
        let log = tracked(&target);
        target.clear();
        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Remove, vec![], vec![1, 2, 3], -1, 0)]);
    }

    #[test]
    fn can_modify_list_while_enumerating_snapshot() {
        let target = FerroList::from_items([1, 2, 3]);
        let mut seen = Vec::new();
        target.for_each(|i| {
            seen.push(*i);
            target.remove(i);
        });
        assert_eq!(seen, vec![1, 2, 3]);
        assert!(target.is_empty());
    }
}
