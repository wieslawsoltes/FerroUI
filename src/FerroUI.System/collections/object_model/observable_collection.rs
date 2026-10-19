//! `ObservableCollection<T>` (`System.Collections.ObjectModel.ObservableCollection<T>`).

use crate::collections::specialized::{CollectionChange, INotifyCollectionChanged, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::component_model::INotifyPropertyChanged;
use crate::events::{Event, HandlerList};
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;

/// A handler of the typed change notifications of a collection (`NotifyCollectionChangedEventHandler`).
pub type CollectionChangedHandler<T> = dyn for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, T>);

/// The name of the property of the indexer (`"Item[]"`).
const INDEXER_NAME: &str = "Item[]";

struct Inner<T> {
    items: RefCell<Vec<T>>,
    collection_changed: HandlerList<CollectionChangedHandler<T>>,
    untyped_collection_changed: Event<CollectionChange>,
    property_changed: Event<str>,
    /// The depth of the notifications being raised (`_blockReentrancyCount`).
    block_reentrancy_count: Cell<usize>,
    /// Notifications are held back (the overrides of `OnCollectionChanged`/`OnPropertyChanged` of
    /// subclasses such as DynamicData's `ObservableCollectionExtended`).
    notifications_suspended: Cell<bool>,
}

/// A list that notifies its changes (`ObservableCollection<T>`): a shared handle, as the object
/// of .NET is a reference. Each change raises `PropertyChanged` for `Count` and `Item[]` as it
/// applies, then `CollectionChanged`, in .NET's order.
pub struct ObservableCollection<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Clone for ObservableCollection<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> Default for ObservableCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PartialEq for ObservableCollection<T> {
    /// The same collection (reference equality, as for the object of .NET).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: Clone + PartialEq + 'static> ObservableCollection<T> {
    /// An empty collection.
    pub fn new() -> Self {
        Self::from_items(Vec::new())
    }

    /// A collection of the items (`ObservableCollection(IEnumerable<T>)`).
    pub fn from_items(items: Vec<T>) -> Self {
        Self {
            inner: Rc::new(Inner {
                items: RefCell::new(items),
                collection_changed: HandlerList::new(),
                untyped_collection_changed: Event::new(),
                property_changed: Event::new(),
                block_reentrancy_count: Cell::new(0),
                notifications_suspended: Cell::new(false),
            }),
        }
    }

    /// Whether `self` and `other` are the same collection.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// Adds a handler of the typed change notifications; returns the token that removes it
    /// (`CollectionChanged +=`).
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        self.inner.collection_changed.add(handler)
    }

    /// Removes a handler (`CollectionChanged -=`).
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.inner.collection_changed.remove(token)
    }

    /// The number of items (`Count`).
    pub fn count(&self) -> usize {
        self.inner.items.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.items.borrow().is_empty()
    }

    /// The item at `index` (the indexer's getter).
    ///
    /// # Panics
    /// When `index` is out of range (`ArgumentOutOfRangeException`).
    pub fn get(&self, index: usize) -> T {
        self.inner.items.borrow()[index].clone()
    }

    /// The items, borrowed.
    pub fn items(&self) -> Ref<'_, Vec<T>> {
        self.inner.items.borrow()
    }

    /// A copy of the items.
    pub fn to_vec(&self) -> Vec<T> {
        self.inner.items.borrow().clone()
    }

    /// Whether the collection has `item` (`Contains`).
    pub fn contains(&self, item: &T) -> bool {
        self.inner.items.borrow().contains(item)
    }

    /// The index of `item` (`IndexOf`).
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.inner.items.borrow().iter().position(|candidate| candidate == item)
    }

    /// Adds an item at the end (`Add`).
    pub fn add(&self, item: T) {
        let index = self.count();
        self.insert(index, item);
    }

    /// Inserts an item (`InsertItem`).
    pub fn insert(&self, index: usize, item: T) {
        self.check_reentrancy();
        self.inner.items.borrow_mut().insert(index, item.clone());
        self.on_count_property_changed();
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Add, std::slice::from_ref(&item), &[], index as i32, -1);
    }

    /// Replaces the item at `index` (the indexer's setter, `SetItem`).
    pub fn set(&self, index: usize, item: T) {
        self.check_reentrancy();
        let original = std::mem::replace(&mut self.inner.items.borrow_mut()[index], item.clone());
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Replace, std::slice::from_ref(&item), std::slice::from_ref(&original), index as i32, index as i32);
    }

    /// Removes the item at `index` (`RemoveItem`).
    pub fn remove_at(&self, index: usize) {
        self.check_reentrancy();
        let removed = self.inner.items.borrow_mut().remove(index);
        self.on_count_property_changed();
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Remove, &[], std::slice::from_ref(&removed), -1, index as i32);
    }

    /// Removes `item`; false when it is not there (`Remove`).
    pub fn remove(&self, item: &T) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Moves the item at `old_index` to `new_index` (`Move`, `MoveItem`).
    pub fn move_item(&self, old_index: usize, new_index: usize) {
        self.check_reentrancy();
        let item = {
            let mut items = self.inner.items.borrow_mut();
            let item = items.remove(old_index);
            items.insert(new_index, item.clone());
            item
        };
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Move, std::slice::from_ref(&item), std::slice::from_ref(&item), new_index as i32, old_index as i32);
    }

    /// Removes every item (`ClearItems`): a reset.
    pub fn clear(&self) {
        self.check_reentrancy();
        self.inner.items.borrow_mut().clear();
        self.on_count_property_changed();
        self.on_indexer_property_changed();
        self.on_collection_reset();
    }

    /// The items to change without notifications (the protected `Items` of `Collection<T>`), for
    /// a collection that raises its own notifications afterwards.
    pub fn items_mut_silent(&self) -> RefMut<'_, Vec<T>> {
        self.inner.items.borrow_mut()
    }

    /// Holds back the notifications until `false` (the overrides of `OnCollectionChanged` and
    /// `OnPropertyChanged` of a collection that suspends them).
    pub fn set_notifications_suspended(&self, suspended: bool) {
        self.inner.notifications_suspended.set(suspended);
    }

    pub fn notifications_suspended(&self) -> bool {
        self.inner.notifications_suspended.get()
    }

    /// Panics on a change made by a handler of a notification while more than one handler listens
    /// (`CheckReentrancy`, `InvalidOperationException`).
    pub fn check_reentrancy(&self) {
        if self.inner.block_reentrancy_count.get() > 0 && self.inner.collection_changed.len() + usize::from(self.inner.untyped_collection_changed.has_handlers()) > 1 {
            panic!("Cannot change ObservableCollection during a CollectionChanged event.");
        }
    }

    /// Raises `PropertyChanged` for `name` (`OnPropertyChanged`).
    pub fn on_property_changed(&self, name: &str) {
        if self.inner.notifications_suspended.get() {
            return;
        }
        self.inner.property_changed.raise(name);
    }

    /// `OnCountPropertyChanged`.
    pub fn on_count_property_changed(&self) {
        self.on_property_changed("Count");
    }

    /// `OnIndexerPropertyChanged`.
    pub fn on_indexer_property_changed(&self) {
        self.on_property_changed(INDEXER_NAME);
    }

    /// A reset (`OnCollectionReset`).
    pub fn on_collection_reset(&self) {
        self.on_collection_changed(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
    }

    /// Raises `CollectionChanged` (`OnCollectionChanged`), with the typed and the untyped
    /// handlers, blocking re-entrant changes meanwhile (`BlockReentrancy`).
    pub fn on_collection_changed(&self, action: NotifyCollectionChangedAction, new_items: &[T], old_items: &[T], new_starting_index: i32, old_starting_index: i32) {
        if self.inner.notifications_suspended.get() {
            return;
        }
        self.inner.block_reentrancy_count.set(self.inner.block_reentrancy_count.get() + 1);
        struct Unblock<'a>(&'a Cell<usize>);
        impl Drop for Unblock<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() - 1);
            }
        }
        let _unblock = Unblock(&self.inner.block_reentrancy_count);
        let args = NotifyCollectionChangedEventArgs { action, new_items, old_items, new_starting_index, old_starting_index };
        for (_, handler) in self.inner.collection_changed.snapshot().iter() {
            handler(&args);
        }
        self.inner.untyped_collection_changed.raise(&CollectionChange {
            action,
            new_starting_index,
            new_count: new_items.len(),
            old_starting_index,
            old_count: old_items.len(),
        });
    }
}

impl<T> INotifyCollectionChanged for ObservableCollection<T> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.inner.untyped_collection_changed
    }
}

impl<T> INotifyPropertyChanged for ObservableCollection<T> {
    fn property_changed(&self) -> &Event<str> {
        &self.inner.property_changed
    }
}
