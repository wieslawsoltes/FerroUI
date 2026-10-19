//! `ObservableCollectionExtended<T>` (`Binding/ObservableCollectionExtended.cs`).

use super::{IObservableCollection, INotifyCollectionChangedSuspender};
use crate::list::{IExtendedList, IList};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::collections::object_model::{CollectionChangedHandler, ObservableCollection};
use ferroui_system::collections::specialized::{CollectionChange, INotifyCollectionChanged, NotifyCollectionChangedAction};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::{Cell, Ref};
use std::fmt;
use std::rc::Rc;

/// The name of the property of the number of items.
const COUNT_NAME: &str = "Count";
/// The name of the property of the indexer.
const INDEXER_NAME: &str = "Item[]";

struct SuspendState {
    /// `_suspendCount`.
    suspend_count: Cell<bool>,
    /// `_suspendNotifications`.
    suspend_notifications: Cell<bool>,
}

/// An observable collection with ranges, loads and suspended notifications
/// (`ObservableCollectionExtended<T>`), the collection that the binding operators fill.
///
/// It is a shared handle over an [`ObservableCollection<T>`] (the base class upstream), which items
/// controls read and observe: [`collection`](Self::collection) is that collection. The changes made
/// through this handle raise the notifications of the base, except those that are suspended
/// (upstream's overrides of `OnCollectionChanged` and `OnPropertyChanged`).
///
/// Deviation (DEVIATIONS.md, DynamicData): a subclass of .NET intercepts the notifications of every
/// change; here the changes made through [`collection`](Self::collection) directly are not
/// suspended.
pub struct ObservableCollectionExtended<T> {
    collection: ObservableCollection<T>,
    state: Rc<SuspendState>,
}

impl<T> Clone for ObservableCollectionExtended<T> {
    fn clone(&self) -> Self {
        Self { collection: self.collection.clone(), state: self.state.clone() }
    }
}

impl<T> PartialEq for ObservableCollectionExtended<T> {
    /// The same collection (reference equality).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl<T: Clone + PartialEq + 'static> Default for ObservableCollectionExtended<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + fmt::Debug + 'static> fmt::Debug for ObservableCollectionExtended<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.collection.items().iter()).finish()
    }
}

impl<T: Clone + PartialEq + 'static> ObservableCollectionExtended<T> {
    /// An empty collection (`ObservableCollectionExtended()`).
    pub fn new() -> Self {
        Self::from_items(Vec::new())
    }

    /// An empty collection that has room for `capacity` items (`ObservableCollectionExtended(new
    /// List<T>(capacity))`).
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_items(Vec::with_capacity(capacity))
    }

    /// A collection of the items (`ObservableCollectionExtended(IEnumerable<T>)`,
    /// `ObservableCollectionExtended(List<T>)`).
    pub fn from_items(items: Vec<T>) -> Self {
        Self {
            collection: ObservableCollection::from_items(items),
            state: Rc::new(SuspendState { suspend_count: Cell::new(false), suspend_notifications: Cell::new(false) }),
        }
    }

    /// The observable collection of the items (the base class upstream).
    pub fn collection(&self) -> &ObservableCollection<T> {
        &self.collection
    }

    /// Whether `self` and `other` are the same collection.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
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

    /// The item at `index` (the indexer's getter).
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

    /// Adds an item at the end (`Add`).
    pub fn add(&self, item: T) {
        let index = self.count();
        self.insert(index, item);
    }

    /// Inserts an item (`InsertItem`).
    pub fn insert(&self, index: usize, item: T) {
        self.collection.check_reentrancy();
        self.collection.items_mut_silent().insert(index, item.clone());
        self.on_property_changed(COUNT_NAME);
        self.on_property_changed(INDEXER_NAME);
        self.on_collection_changed(NotifyCollectionChangedAction::Add, std::slice::from_ref(&item), &[], index as i32, -1);
    }

    /// Replaces the item at `index` (the indexer's setter, `SetItem`).
    pub fn set(&self, index: usize, item: T) {
        self.collection.check_reentrancy();
        let original = std::mem::replace(&mut self.collection.items_mut_silent()[index], item.clone());
        self.on_property_changed(INDEXER_NAME);
        self.on_collection_changed(NotifyCollectionChangedAction::Replace, std::slice::from_ref(&item), std::slice::from_ref(&original), index as i32, index as i32);
    }

    /// Removes the item at `index` (`RemoveItem`).
    pub fn remove_at(&self, index: usize) {
        self.collection.check_reentrancy();
        let removed = self.collection.items_mut_silent().remove(index);
        self.on_property_changed(COUNT_NAME);
        self.on_property_changed(INDEXER_NAME);
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
        self.collection.check_reentrancy();
        let item = {
            let mut items = self.collection.items_mut_silent();
            let item = items.remove(old_index);
            items.insert(new_index, item.clone());
            item
        };
        self.on_property_changed(INDEXER_NAME);
        self.on_collection_changed(NotifyCollectionChangedAction::Move, std::slice::from_ref(&item), std::slice::from_ref(&item), new_index as i32, old_index as i32);
    }

    /// Removes every item (`ClearItems`): a reset.
    pub fn clear(&self) {
        self.collection.check_reentrancy();
        self.collection.items_mut_silent().clear();
        self.on_property_changed(COUNT_NAME);
        self.on_property_changed(INDEXER_NAME);
        self.on_collection_changed(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
    }

    /// Adds the items at the end, one at a time (`AddRange`).
    pub fn add_range(&self, items: impl IntoIterator<Item = T>) {
        for item in items {
            self.add(item);
        }
    }

    /// Inserts the items from `index`, one at a time (`InsertRange`).
    pub fn insert_range(&self, items: impl IntoIterator<Item = T>, index: usize) {
        for (offset, item) in items.into_iter().enumerate() {
            self.insert(index + offset, item);
        }
    }

    /// Clears the collection and adds the items (`Load`).
    pub fn load(&self, items: impl IntoIterator<Item = T>) {
        self.collection.check_reentrancy();
        self.clear();
        for item in items {
            self.add(item);
        }
    }

    /// Removes `count` items from `index`, one at a time (`RemoveRange`).
    pub fn remove_range(&self, index: usize, count: usize) {
        for _ in 0..count {
            self.remove_at(index);
        }
    }

    /// Holds back the notifications of `Count` until the handle is disposed, which raises `Count`
    /// once if it changed (`SuspendCount`).
    pub fn suspend_count(&self) -> Rc<dyn IDisposable> {
        let count = self.count();
        self.state.suspend_count.set(true);
        let this = self.clone();
        Disposable::create(move || {
            this.state.suspend_count.set(false);
            if this.count() != count {
                this.on_property_changed(COUNT_NAME);
            }
        })
    }

    /// Holds back the notifications of changes and of `Count` until the handle is disposed, which
    /// raises `Count` and a reset (`SuspendNotifications`).
    pub fn suspend_notifications(&self) -> Rc<dyn IDisposable> {
        self.state.suspend_count.set(true);
        self.state.suspend_notifications.set(true);
        let this = self.clone();
        Disposable::create(move || {
            this.state.suspend_count.set(false);
            this.state.suspend_notifications.set(false);
            this.on_property_changed(COUNT_NAME);
            this.on_collection_changed(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
        })
    }

    /// Raises `PropertyChanged`, unless it is `Count` and the count is suspended
    /// (`OnPropertyChanged`).
    pub fn on_property_changed(&self, name: &str) {
        if self.state.suspend_count.get() && name == COUNT_NAME {
            return;
        }
        self.collection.on_property_changed(name);
    }

    /// Raises `CollectionChanged`, unless the notifications are suspended (`OnCollectionChanged`).
    pub fn on_collection_changed(&self, action: NotifyCollectionChangedAction, new_items: &[T], old_items: &[T], new_starting_index: i32, old_starting_index: i32) {
        if self.state.suspend_notifications.get() {
            return;
        }
        self.collection.on_collection_changed(action, new_items, old_items, new_starting_index, old_starting_index);
    }
}

impl<T> INotifyCollectionChanged for ObservableCollectionExtended<T> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        self.collection.collection_changed()
    }
}

impl<T> INotifyPropertyChanged for ObservableCollectionExtended<T> {
    fn property_changed(&self) -> &Event<str> {
        self.collection.property_changed()
    }
}

impl<T: Clone + PartialEq + 'static> INotifyCollectionChangedSuspender for ObservableCollectionExtended<T> {
    fn suspend_count(&self) -> Rc<dyn IDisposable> {
        ObservableCollectionExtended::suspend_count(self)
    }

    fn suspend_notifications(&self) -> Rc<dyn IDisposable> {
        ObservableCollectionExtended::suspend_notifications(self)
    }
}

impl<T: Clone + PartialEq + 'static> IObservableCollection<T> for ObservableCollectionExtended<T> {
    fn count(&self) -> usize {
        ObservableCollectionExtended::count(self)
    }

    fn get(&self, index: usize) -> T {
        ObservableCollectionExtended::get(self, index)
    }

    fn set(&self, index: usize, item: T) {
        ObservableCollectionExtended::set(self, index, item);
    }

    fn insert(&self, index: usize, item: T) {
        ObservableCollectionExtended::insert(self, index, item);
    }

    fn remove_at(&self, index: usize) {
        ObservableCollectionExtended::remove_at(self, index);
    }

    fn clear(&self) {
        ObservableCollectionExtended::clear(self);
    }

    fn to_vec(&self) -> Vec<T> {
        ObservableCollectionExtended::to_vec(self)
    }

    fn add(&self, item: T) {
        ObservableCollectionExtended::add(self, item);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        ObservableCollectionExtended::index_of(self, item)
    }

    fn remove(&self, item: &T) -> bool {
        ObservableCollectionExtended::remove(self, item)
    }

    fn load(&self, items: Vec<T>) {
        ObservableCollectionExtended::load(self, items);
    }

    fn move_item(&self, old_index: usize, new_index: usize) {
        ObservableCollectionExtended::move_item(self, old_index, new_index);
    }

    /// The changes applied as to an `IExtendedList<T>` (ranges at once).
    fn clone_changes(&self, changes: &crate::list::ChangeSet<T>) {
        crate::list::clone_changes(&mut self.clone(), changes);
    }
}

/// The collection as a list of DynamicData (`IList<T>`, `IExtendedList<T>`): what `ListEx` and
/// `SortAndBind` apply changes to.
impl<T: Clone + PartialEq + 'static> IList<T> for ObservableCollectionExtended<T> {
    fn count(&self) -> usize {
        ObservableCollectionExtended::count(self)
    }

    fn get(&self, index: usize) -> T {
        ObservableCollectionExtended::get(self, index)
    }

    fn set(&mut self, index: usize, item: T) {
        ObservableCollectionExtended::set(self, index, item);
    }

    fn insert(&mut self, index: usize, item: T) {
        ObservableCollectionExtended::insert(self, index, item);
    }

    fn remove_at(&mut self, index: usize) {
        ObservableCollectionExtended::remove_at(self, index);
    }

    fn clear(&mut self) {
        ObservableCollectionExtended::clear(self);
    }

    fn add(&mut self, item: T) {
        ObservableCollectionExtended::add(self, item);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        ObservableCollectionExtended::index_of(self, item)
    }

    fn remove(&mut self, item: &T) -> bool {
        ObservableCollectionExtended::remove(self, item)
    }

    fn add_range(&mut self, items: Vec<T>) {
        ObservableCollectionExtended::add_range(self, items);
    }

    fn add_or_insert_range(&mut self, items: Vec<T>, index: Option<usize>) {
        match index {
            Some(index) => ObservableCollectionExtended::insert_range(self, items, index),
            None => ObservableCollectionExtended::add_range(self, items),
        }
    }

    fn remove_range(&mut self, index: usize, count: usize) {
        ObservableCollectionExtended::remove_range(self, index, count);
    }

    fn supports_ranges(&self) -> bool {
        true
    }

    fn move_item(&mut self, original: usize, destination: usize) {
        ObservableCollectionExtended::move_item(self, original, destination);
    }
}

impl<T: Clone + PartialEq + 'static> IExtendedList<T> for ObservableCollectionExtended<T> {
    fn insert_range(&mut self, items: Vec<T>, index: usize) {
        ObservableCollectionExtended::insert_range(self, items, index);
    }

    fn move_index(&mut self, original: usize, destination: usize) {
        ObservableCollectionExtended::move_item(self, original, destination);
    }
}
