//! `ObservableCollection<T>` (`System.Collections.ObjectModel.ObservableCollection<T>`).

use crate::collections::specialized::{CollectionChange, INotifyCollectionChanged, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::component_model::INotifyPropertyChanged;
use crate::events::{Event, HandlerList};
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A handler of the typed change notifications of a collection (`NotifyCollectionChangedEventHandler`).
pub type CollectionChangedHandler<T> = dyn for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, T>);

/// The name of the property of the indexer (`"Item[]"`).
const INDEXER_NAME: &str = "Item[]";

struct Inner<T> {
    items: RefCell<Vec<T>>,
    version: Cell<i32>,
    capacity: Cell<usize>,
    policy: SlotPolicy<T>,
    comparer: Option<for<'a> fn(&CollectionItemView<'a, T>, &T) -> bool>,
    search_array: RefCell<Option<Rc<SearchArray<T>>>>,
    raw_edit: Cell<bool>,
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

/// A CLR value-type comparison can retain a byref receiver into an array slot.
/// Read fields through this view before/after callbacks instead of holding a Rust
/// reference into mutable storage. Reference-type representations use PartialEq.
pub struct CollectionItemView<'a, T> {
    collection: &'a ObservableCollection<T>,
    array: &'a SearchArray<T>,
    index: usize,
}
impl<T: Clone + PartialEq + 'static> CollectionItemView<'_, T> {
    pub fn read(&self) -> T {
        self.collection.slot_value(self.array, self.index)
    }
}

// Rust representation cannot infer CLR RuntimeHelpers.IsReferenceOrContainsReferences<T>
// or default(T). Library constructors declare the policy; unspecified native types
// fail explicitly only when a search observes a cleared/retained out-of-Count slot.
enum SlotPolicy<T> {
    Unspecified,
    Managed(fn() -> T),
    Unmanaged,
}
#[derive(Clone)]
enum SearchSlot<T> {
    Value(T),
    Default,
    Unspecified,
}
struct SearchArray<T> {
    active: Cell<usize>,
    limit: Cell<usize>,
    tails: RefCell<BTreeMap<usize, T>>,
    retired: RefCell<Option<Vec<SearchSlot<T>>>>,
}
impl<T> SearchArray<T> {
    fn new() -> Self {
        Self { active: Cell::new(0), limit: Cell::new(0), tails: RefCell::new(BTreeMap::new()), retired: RefCell::new(None) }
    }
}
struct SearchLease<T>(Rc<SearchArray<T>>);
impl<T> Drop for SearchLease<T> {
    fn drop(&mut self) {
        self.0.active.set(self.0.active.get() - 1);
        if self.0.active.get() == 0 {
            self.0.limit.set(0);
            let tails = std::mem::take(&mut *self.0.tails.borrow_mut());
            let retired = self.0.retired.borrow_mut().take();
            drop(tails);
            drop(retired);
        }
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
        Self::with_policy(items, SlotPolicy::Unspecified)
    }

    /// CLR reference / reference-containing value storage. The Rust Default implementation
    /// must represent CLR default(T), not construct a new object. In particular Option<Rc<_>>
    /// supplies the required null. No reflection or needs_drop heuristic selects this policy.
    pub fn new_managed() -> Self
    where
        T: Default,
    {
        Self::from_items_managed(Vec::new())
    }
    pub fn from_items_managed(items: Vec<T>) -> Self
    where
        T: Default,
    {
        Self::with_policy(items, SlotPolicy::Managed(T::default))
    }

    /// CLR value storage containing no managed references: RemoveAt and Clear leave old
    /// array slots intact. No Rust Default bound is needed for this policy.
    pub fn new_unmanaged() -> Self {
        Self::from_items_unmanaged(Vec::new())
    }
    pub fn from_items_unmanaged(items: Vec<T>) -> Self {
        Self::with_policy(items, SlotPolicy::Unmanaged)
    }

    /// Alias-sensitive CLR value Equals: receiver reads use the captured array,
    /// and the query is copied once at entry as the source value parameter is.
    pub fn from_items_managed_with_comparer(items: Vec<T>, compare: for<'a> fn(&CollectionItemView<'a, T>, &T) -> bool) -> Self
    where
        T: Default,
    {
        let mut collection = Self::from_items_managed(items);
        Rc::get_mut(&mut collection.inner).expect("fresh collection").comparer = Some(compare);
        collection
    }
    pub fn from_items_unmanaged_with_comparer(items: Vec<T>, compare: for<'a> fn(&CollectionItemView<'a, T>, &T) -> bool) -> Self {
        let mut collection = Self::from_items_unmanaged(items);
        Rc::get_mut(&mut collection.inner).expect("fresh collection").comparer = Some(compare);
        collection
    }

    fn with_policy(items: Vec<T>, policy: SlotPolicy<T>) -> Self {
        let capacity = items.len();
        Self {
            inner: Rc::new(Inner {
                items: RefCell::new(items),
                version: Cell::new(0),
                capacity: Cell::new(capacity),
                policy,
                comparer: None,
                search_array: RefCell::new(None),
                raw_edit: Cell::new(false),
                // Both views represent one managed event. Global ordered tokens avoid
                // an extra counter allocation per collection and order the merged snapshot.
                collection_changed: HandlerList::new(),
                untyped_collection_changed: Event::new(),
                property_changed: Event::new(),
                block_reentrancy_count: Cell::new(0),
                notifications_suspended: Cell::new(false),
            }),
        }
    }

    // Raw native Vec editing is not a CLR IList mutation protocol. Outside active
    // searches it starts a new logical buffer, with the resulting native capacity.
    fn sync_raw_edit(&self) {
        if self.inner.raw_edit.get() {
            let capacity = self.inner.items.borrow().capacity();
            self.inner.capacity.set(capacity);
            self.inner.raw_edit.set(false);
        }
    }

    fn active_array(&self) -> Option<Rc<SearchArray<T>>> {
        self.inner.search_array.borrow().as_ref().filter(|a| a.active.get() > 0).cloned()
    }

    fn slot(&self, array: &SearchArray<T>, index: usize) -> SearchSlot<T> {
        if let Some(retired) = &*array.retired.borrow() {
            return retired[index].clone();
        }
        let items = self.inner.items.borrow();
        if let Some(value) = items.get(index) {
            return SearchSlot::Value(value.clone());
        }
        drop(items);
        match &self.inner.policy {
            SlotPolicy::Managed(_) => SearchSlot::Default,
            SlotPolicy::Unmanaged => SearchSlot::Value(array.tails.borrow().get(&index).expect("retained CLR array slot").clone()),
            SlotPolicy::Unspecified => SearchSlot::Unspecified,
        }
    }

    fn slot_value(&self, array: &SearchArray<T>, index: usize) -> T {
        match self.slot(array, index) {
            SearchSlot::Value(value) => value,
            SearchSlot::Default => match &self.inner.policy {
                SlotPolicy::Managed(default) => default(),
                _ => unreachable!("managed default slot"),
            },
            SearchSlot::Unspecified => {
                panic!("CLR cleared-slot policy is unspecified; select new_managed or new_unmanaged for this representation")
            }
        }
    }

    fn grown_capacity(current: usize, required: usize) -> usize {
        const MAX_ARRAY_LENGTH: usize = 0x7fffffc7;
        (if current == 0 { 4 } else { current.saturating_mul(2) }).min(MAX_ARRAY_LENGTH).max(required)
    }

    fn grow_for(&self, required: usize) {
        if required <= self.inner.capacity.get() {
            return;
        }
        let previous = self.inner.search_array.borrow_mut().take();
        if let Some(array) = previous.filter(|a| a.active.get() > 0) {
            let count = self.inner.items.borrow().len();
            let slots = (0..count.max(array.limit.get())).map(|i| self.slot(&array, i)).collect();
            *array.retired.borrow_mut() = Some(slots);
        }
        self.inner.capacity.set(Self::grown_capacity(self.inner.capacity.get(), required));
    }

    fn remove_storage(&self, index: usize) -> T {
        let mut items = self.inner.items.borrow_mut();
        assert!(index < items.len(), "index out of range");
        if let Some(array) =
            self.active_array().filter(|a| matches!(self.inner.policy, SlotPolicy::Unmanaged) && items.len() - 1 < a.limit.get())
        {
            array.tails.borrow_mut().insert(items.len() - 1, items.last().expect("nonempty").clone());
        }
        items.remove(index)
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
        self.sync_raw_edit();
        self.inner.items.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.sync_raw_edit();
        self.inner.items.borrow().is_empty()
    }

    /// The item at `index` (the indexer's getter).
    ///
    /// # Panics
    /// When `index` is out of range (`ArgumentOutOfRangeException`).
    pub fn get(&self, index: usize) -> T {
        self.sync_raw_edit();
        self.inner.items.borrow()[index].clone()
    }

    /// The items, borrowed.
    pub fn items(&self) -> Ref<'_, Vec<T>> {
        self.sync_raw_edit();
        self.inner.items.borrow()
    }

    /// A copy of the items.
    pub fn to_vec(&self) -> Vec<T> {
        self.sync_raw_edit();
        self.inner.items.borrow().clone()
    }

    /// An owning interface-style enumerator. When nonempty, each next checks the
    /// version captured now, including after reaching the end. No collection borrow
    /// survives a call. When empty at creation, it stays empty without retaining the
    /// collection, as ICollection<T>.GetEnumerator backed by List<T> does.
    pub fn iter(&self) -> ObservableCollectionIter<T> {
        ObservableCollectionIter { collection: (!self.is_empty()).then(|| self.clone()), version: self.inner.version.get(), index: 0, ended: false }
    }

    fn bump_version(&self) { self.inner.version.set(self.inner.version.get().wrapping_add(1)); }

    /// Whether the collection has `item` (`Contains`).
    pub fn contains(&self, item: &T) -> bool {
        self.index_of(item).is_some()
    }

    /// Search a captured source array and Count. Comparisons run outside storage
    /// borrows; in-place mutations stay visible, while growth retires the old array.
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.sync_raw_edit();
        let count = self.count();
        if count == 0 {
            return None;
        }
        let array = {
            let mut current = self.inner.search_array.borrow_mut();
            current.get_or_insert_with(|| Rc::new(SearchArray::new())).clone()
        };
        array.active.set(array.active.get() + 1);
        array.limit.set(array.limit.get().max(count));
        let _lease = SearchLease(array.clone());
        let copied_query = self.inner.comparer.map(|_| item.clone());
        for index in 0..count {
            let view = CollectionItemView { collection: self, array: &array, index };
            let equal = if let Some(compare) = self.inner.comparer {
                compare(&view, copied_query.as_ref().expect("value query"))
            } else {
                view.read() == *item
            };
            if equal {
                return Some(index);
            }
        }
        None
    }
    /// Adds an item at the end (`Add`).
    pub fn add(&self, item: T) {
        let index = self.count();
        self.insert(index, item);
    }

    /// Inserts an item (`InsertItem`).
    pub fn insert(&self, index: usize, item: T) {
        self.check_reentrancy();
        self.sync_raw_edit();
        assert!(index <= self.count(), "index out of range");
        self.grow_for(self.count().checked_add(1).expect("collection capacity overflow"));
        self.inner.items.borrow_mut().insert(index, item.clone());
        self.bump_version();
        self.on_count_property_changed();
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Add, std::slice::from_ref(&item), &[], index as i32, -1);
    }

    /// Replaces the item at `index` (the indexer's setter, `SetItem`).
    pub fn set(&self, index: usize, item: T) {
        self.check_reentrancy();
        self.sync_raw_edit();
        let original = std::mem::replace(&mut self.inner.items.borrow_mut()[index], item.clone());
        self.bump_version();
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Replace, std::slice::from_ref(&item), std::slice::from_ref(&original), index as i32, index as i32);
    }

    /// Removes the item at `index` (`RemoveItem`).
    pub fn remove_at(&self, index: usize) {
        self.check_reentrancy();
        self.sync_raw_edit();
        let removed = self.remove_storage(index);
        self.bump_version();
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
        self.sync_raw_edit();
        let item = self.remove_storage(old_index);
        self.bump_version();
        self.inner.items.borrow_mut().insert(new_index, item.clone());
        self.bump_version();
        self.on_indexer_property_changed();
        self.on_collection_changed(NotifyCollectionChangedAction::Move, std::slice::from_ref(&item), std::slice::from_ref(&item), new_index as i32, old_index as i32);
    }

    /// Removes every item (`ClearItems`): a reset.
    pub fn clear(&self) {
        self.check_reentrancy();
        self.sync_raw_edit();
        if let Some(array) = self.active_array().filter(|_| matches!(self.inner.policy, SlotPolicy::Unmanaged)) {
            let items = self.inner.items.borrow();
            let mut tails = array.tails.borrow_mut();
            for (i, value) in items.iter().take(array.limit.get()).enumerate() { tails.insert(i, value.clone()); }
        }
        let mut removed = std::mem::take(&mut *self.inner.items.borrow_mut());
        self.bump_version();
        // Drop user values without a collection borrow; reuse capacity if no native
        // destructor inserted new values while they were being released.
        removed.clear();
        {
            let mut items = self.inner.items.borrow_mut();
            if items.is_empty() && items.capacity() < removed.capacity() { std::mem::swap(&mut *items, &mut removed); }
        }
        self.on_count_property_changed();
        self.on_indexer_property_changed();
        self.on_collection_reset();
    }

    /// The items to change without notifications (the protected `Items` of `Collection<T>`), for
    /// a collection that raises its own notifications afterwards.
    /// Acquiring this raw edit guard invalidates existing iterators, even if no
    /// write follows. Do not create or advance iterators while retaining the guard.
    /// Raw editing during Equals/search is explicitly rejected. After editing, the
    /// logical source-array capacity follows the native Vec capacity.
    pub fn items_mut_silent(&self) -> RefMut<'_, Vec<T>> {
        self.sync_raw_edit();
        assert!(self.active_array().is_none(), "raw Vec editing during a CLR array search is unsupported; use collection mutations");
        self.inner.search_array.borrow_mut().take();
        let items = self.inner.items.borrow_mut();
        self.inner.raw_edit.set(true);
        self.bump_version();
        items
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
        if self.inner.block_reentrancy_count.get() > 0 && self.inner.collection_changed.len() + self.inner.untyped_collection_changed.handler_count() > 1 {
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
        // Take BOTH snapshots before invoking any handler. Adding/removing a
        // handler through the other view affects the next notification only.
        let typed = self.inner.collection_changed.snapshot();
        let untyped = self.inner.untyped_collection_changed.snapshot();
        let change = CollectionChange {
            action,
            new_starting_index,
            new_count: new_items.len(),
            old_starting_index,
            old_count: old_items.len(),
        };
        let (mut t, mut u) = (0, 0);
        while t < typed.len() || u < untyped.len() {
            if u == untyped.len() || (t < typed.len() && typed[t].0 < untyped[u].0) {
                typed[t].1(&args);
                t += 1;
            } else {
                untyped[u].1(&change);
                u += 1;
            }
        }
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

/// Owned traversal corresponding to the interface enumerator of List<T>.
/// Empty collections produce a permanently empty, non-owning iterator.
/// Rust next returns an owned clone; CLR Current/Reset/Dispose interfaces are not
/// represented by Iterator. Clone copies traversal state, not collection contents.
#[derive(Clone)]
pub struct ObservableCollectionIter<T> {
    collection: Option<ObservableCollection<T>>,
    version: i32,
    index: usize,
    ended: bool,
}
impl<T: Clone + PartialEq + 'static> Iterator for ObservableCollectionIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let Some(collection) = &self.collection else { return None };
        assert_eq!(self.version, collection.inner.version.get(), "Collection was modified; enumeration operation may not execute.");
        if self.ended {
            return None;
        }
        let value = collection.inner.items.borrow().get(self.index).cloned();
        if value.is_some() {
            self.index += 1;
        } else {
            self.ended = true;
        }
        value
    }
}

#[cfg(test)]
mod iterator_tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    #[test]
    fn iteration_releases_storage_and_checks_mutations_after_end() {
        let items = ObservableCollection::from_items(vec![1, 2]);
        let mut iterator = items.iter();
        assert_eq!(iterator.next(), Some(1));
        assert_eq!(iterator.next(), Some(2));
        assert_eq!(iterator.next(), None);
        items.add(3);
        assert!(catch_unwind(AssertUnwindSafe(|| iterator.next())).is_err());
    }
    #[test]
    fn equal_set_and_wrapping_version_invalidate_but_empty_iterator_stays_empty() {
        let items = ObservableCollection::from_items(vec![1]);
        items.inner.version.set(i32::MAX);
        let mut iterator = items.iter();
        items.set(0, 1);
        assert_eq!(items.inner.version.get(), i32::MIN);
        assert!(catch_unwind(AssertUnwindSafe(|| iterator.next())).is_err());
        items.clear();
        let mut empty = items.iter();
        items.clear();
        items.add(1);
        assert_eq!(empty.next(), None);
    }
    #[test]
    fn empty_iterator_does_not_retain_collection_but_nonempty_iterator_does() {
        let empty = ObservableCollection::<i32>::new();
        let weak = Rc::downgrade(&empty.inner);
        let mut iterator = empty.iter();
        drop(empty);
        assert!(weak.upgrade().is_none());
        assert_eq!(iterator.next(), None);
        let items = ObservableCollection::from_items(vec![1]);
        let weak = Rc::downgrade(&items.inner);
        let iterator = items.iter();
        drop(items);
        assert!(weak.upgrade().is_some());
        drop(iterator);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn mutation_is_visible_to_iterators_before_property_handlers_and_despite_failure() {
        let items = ObservableCollection::from_items(vec![1]);
        let iterator = Rc::new(RefCell::new(items.iter()));
        let probe = iterator.clone();
        items.property_changed().add(Rc::new(move |_| {
            assert!(catch_unwind(AssertUnwindSafe(|| probe.borrow_mut().next())).is_err());
            panic!("handler");
        }));
        assert!(catch_unwind(AssertUnwindSafe(|| items.add(2))).is_err());
        assert_eq!(items.count(), 2);
        let mut original = items.iter();
        let mut copy = original.clone();
        assert_eq!(original.next(), Some(1));
        assert_eq!(copy.next(), Some(1));
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    #[derive(Clone)]
    struct Item {
        id: i32,
        callback: Option<Rc<dyn Fn()>>,
    }
    impl PartialEq for Item {
        fn eq(&self, other: &Self) -> bool {
            if let Some(callback) = &self.callback {
                callback();
            }
            self.id == other.id
        }
    }
    fn item(id: i32) -> Rc<Item> {
        Rc::new(Item { id, callback: None })
    }
    #[test]
    fn in_place_writes_are_visible_but_growth_keeps_old_array() {
        for grow in [false, true] {
            let collection = Rc::new(ObservableCollection::new_managed());
            let weak = Rc::downgrade(&collection);
            collection.add(Some(Rc::new(Item {
                id: 1,
                callback: Some(Rc::new(move || {
                    let c = weak.upgrade().unwrap();
                    if grow {
                        for _ in 0..8 {
                            c.add(Some(item(9)));
                        }
                    }
                    c.set(1, Some(item(9)));
                })),
            })));
            collection.add(Some(item(2)));
            assert_eq!(collection.index_of(&Some(item(9))), if grow { None } else { Some(1) });
        }
    }
    #[test]
    fn reference_clear_and_remove_reset_slots_and_release_search_state_on_error() {
        let collection = Rc::new(ObservableCollection::new_managed());
        let weak = Rc::downgrade(&collection);
        collection.add(Some(Rc::new(Item { id: 1, callback: Some(Rc::new(move || weak.upgrade().unwrap().clear())) })));
        collection.add(Some(item(2)));
        assert_eq!(collection.index_of(&Some(item(2))), None);
        assert!(collection.is_empty());
        let collection = Rc::new(ObservableCollection::new_managed());
        let weak = Rc::downgrade(&collection);
        collection.add(Some(Rc::new(Item {
            id: 1,
            callback: Some(Rc::new(move || {
                weak.upgrade().unwrap().add(Some(item(9)));
                panic!("comparison");
            })),
        })));
        assert!(catch_unwind(AssertUnwindSafe(|| collection.contains(&Some(item(2))))).is_err());
        assert_eq!(collection.inner.search_array.borrow().as_ref().unwrap().active.get(), 0);
        collection.items_mut_silent().clear();
        assert!(collection.is_empty());
    }
    #[test]
    fn no_reference_values_retain_tails_and_constructor_capacity_is_count() {
        let collection = ObservableCollection::from_items_unmanaged(vec![1, 2]);
        assert_eq!(collection.inner.capacity.get(), 2);
        let array = Rc::new(SearchArray::new());
        array.active.set(1);
        array.limit.set(2);
        *collection.inner.search_array.borrow_mut() = Some(array.clone());
        collection.remove_at(0);
        assert!(matches!(collection.slot(&array, 1), SearchSlot::Value(2)));
        collection.clear();
        assert!(matches!(collection.slot(&array, 0), SearchSlot::Value(2)));
        assert!(matches!(collection.slot(&array, 1), SearchSlot::Value(2)));
        let _lease = SearchLease(array);
        assert_eq!(ObservableCollection::<i32>::grown_capacity(0, 1), 4);
        assert_eq!(ObservableCollection::<i32>::grown_capacity(4, 5), 8);
        assert_eq!(ObservableCollection::<i32>::grown_capacity(0x7fffffc6, 0x7fffffc7), 0x7fffffc7);
    }
    #[test]
    fn undeclared_default_slots_and_raw_guard_are_explicit_limits() {
        let collection = Rc::new(ObservableCollection::new());
        let weak = Rc::downgrade(&collection);
        collection.add(Item { id: 1, callback: Some(Rc::new(move || weak.upgrade().unwrap().clear())) });
        collection.add(Item { id: 2, callback: None });
        let error = catch_unwind(AssertUnwindSafe(|| collection.index_of(&Item { id: 2, callback: None }))).unwrap_err();
        let text = error.downcast_ref::<&str>().copied().or_else(|| error.downcast_ref::<String>().map(String::as_str)).unwrap();
        assert!(text.contains("policy is unspecified"));
        let collection = Rc::new(ObservableCollection::new());
        let weak = Rc::downgrade(&collection);
        collection.add(Item { id: 1, callback: Some(Rc::new(move || weak.upgrade().unwrap().items_mut_silent().clear())) });
        assert!(catch_unwind(AssertUnwindSafe(|| collection.contains(&Item { id: 2, callback: None }))).is_err());
    }
}
