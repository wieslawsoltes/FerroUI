use super::IFerroListItemValidator;
use crate::data::model::{CollectionChange, Event, INotifyCollectionChanged, INotifyPropertyChanged};
use crate::utilities::{HandlerList, WeakEventSender};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

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
    /// The untyped form of the collection changed event
    /// ([`INotifyCollectionChanged`]), raised after the typed handlers.
    untyped_collection_changed: Event<CollectionChange>,
    property_changed: Event<str>,
    validator: RefCell<Option<ListValidator<T>>>,
    reset_behavior: Cell<ResetBehavior>,
}

/// The validator of a list: one set through
/// [`set_validate`](FerroList::set_validate), whose routine can be read back
/// and replaced, or any other.
enum ListValidator<T> {
    Item(Rc<ItemValidator<T>>),
    Other(Rc<dyn IFerroListItemValidator<T>>),
}

impl<T> Clone for ListValidator<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Item(validator) => Self::Item(validator.clone()),
            Self::Other(validator) => Self::Other(validator.clone()),
        }
    }
}

struct ItemValidator<T> {
    validate: RefCell<Rc<dyn Fn(&T)>>,
}

impl<T> IFerroListItemValidator<T> for ItemValidator<T> {
    fn validate(&self, item: &T) {
        let validate = self.validate.borrow().clone();
        validate(item)
    }
}

/// The weak form of a [`FerroList`] handle.
pub struct WeakFerroList<T>(Weak<FerroListData<T>>);

impl<T> Clone for WeakFerroList<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static> WeakEventSender for FerroList<T> {
    type Weak = WeakFerroList<T>;

    fn downgrade_sender(&self) -> WeakFerroList<T> {
        WeakFerroList(Rc::downgrade(&self.0))
    }

    fn upgrade_sender(weak: &WeakFerroList<T>) -> Option<Self> {
        weak.0.upgrade().map(FerroList)
    }

    fn sender_address(&self) -> usize {
        Rc::as_ptr(&self.0) as *const () as usize
    }
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
            untyped_collection_changed: Event::new(),
            property_changed: Event::new(),
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

    /// Whether anything is subscribed to changes, typed or untyped.
    pub fn has_collection_changed_subscribers(&self) -> bool {
        !self.0.collection_changed.is_empty() || self.0.untyped_collection_changed.has_handlers()
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

    /// The validation routine that is invoked for every item before it is
    /// added to the list.
    pub fn validate(&self) -> Option<Rc<dyn Fn(&T)>>
    where
        T: 'static,
    {
        match self.0.validator.borrow().as_ref() {
            None => None,
            Some(ListValidator::Item(item_validator)) => Some(item_validator.validate.borrow().clone()),
            Some(ListValidator::Other(other)) => {
                let other = other.clone();
                Some(Rc::new(move |item: &T| other.validate(item)))
            }
        }
    }

    /// Sets a validation routine that is invoked for every item before it is
    /// added to the list; it rejects an item by panicking.
    pub fn set_validate(&self, value: Option<Rc<dyn Fn(&T)>>)
    where
        T: 'static,
    {
        let Some(value) = value else {
            self.set_validator(None);
            return;
        };

        let current = self.0.validator.borrow().clone();
        if let Some(ListValidator::Item(item_validator)) = current {
            *item_validator.validate.borrow_mut() = value;
        } else {
            *self.0.validator.borrow_mut() =
                Some(ListValidator::Item(Rc::new(ItemValidator { validate: RefCell::new(value) })));
        }
    }

    /// The validator that is invoked for every item before it is added to the
    /// list.
    pub fn validator(&self) -> Option<Rc<dyn IFerroListItemValidator<T>>>
    where
        T: 'static,
    {
        match self.0.validator.borrow().as_ref()? {
            ListValidator::Item(item_validator) => Some(item_validator.clone()),
            ListValidator::Other(other) => Some(other.clone()),
        }
    }

    pub fn set_validator(&self, value: Option<Rc<dyn IFerroListItemValidator<T>>>) {
        *self.0.validator.borrow_mut() = value.map(ListValidator::Other);
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
        self.validate_item(&value);
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
        self.validate_item(&item);
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
        self.validate_item(&item);
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
            self.validate_item(item);
        }
        self.modify(|list| {
            list.splice(index..index, added.iter().cloned());
        });
        self.notify_add(&added, index);
    }

    /// Moves an item to a new index.
    pub fn move_item(&self, old_index: usize, new_index: usize) {
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
        let moved: Vec<T> = self.get_range(old_index, count);
        let mut insert_at = new_index as isize;
        if new_index > old_index {
            insert_at -= count as isize - 1;
        }
        let insert_at = insert_at as usize;
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
        if self.count() > 0 {
            let old = std::mem::replace(&mut *self.0.items.borrow_mut(), Rc::new(Vec::new()));
            if self.has_collection_changed_subscribers() {
                match self.0.reset_behavior.get() {
                    ResetBehavior::Reset => self.notify(NotifyCollectionChangedEventArgs {
                        action: NotifyCollectionChangedAction::Reset,
                        new_items: &[],
                        old_items: &[],
                        new_starting_index: -1,
                        old_starting_index: -1,
                    }),
                    ResetBehavior::Remove => self.notify(NotifyCollectionChangedEventArgs {
                        action: NotifyCollectionChangedAction::Remove,
                        new_items: &[],
                        old_items: &old,
                        new_starting_index: -1,
                        old_starting_index: 0,
                    }),
                }
            }

            self.notify_count_changed();
        }
    }

    /// Copies the items into `array`, starting at `array_index`.
    ///
    /// # Panics
    /// Panics if `array` is too small, as the managed original throws.
    pub fn copy_to(&self, array: &mut [T], array_index: usize) {
        let items = self.0.items.borrow();
        assert!(
            array_index <= array.len() && array.len() - array_index >= items.len(),
            "Destination array was not long enough. Check the destination index, length, and the array's lower bounds."
        );
        array[array_index..array_index + items.len()].clone_from_slice(&items);
    }

    /// Gets a range of items from the collection.
    pub fn get_range(&self, index: usize, count: usize) -> Vec<T> {
        self.0.items.borrow()[index..index + count].to_vec()
    }

    /// Ensures that the capacity of the list is at least `capacity`.
    pub fn ensure_capacity(&self, capacity: usize) {
        // Adapted from List<T> implementation.
        let current_capacity = self.capacity();
        if current_capacity < capacity {
            let mut new_capacity = if current_capacity == 0 { 4 } else { current_capacity * 2 };

            if new_capacity < capacity {
                new_capacity = capacity;
            }

            self.set_capacity(new_capacity);
        }
    }

    fn validate_item(&self, item: &T) {
        let validator = self.0.validator.borrow().clone();
        match validator {
            None => {}
            Some(ListValidator::Item(validator)) => validator.validate(item),
            Some(ListValidator::Other(validator)) => validator.validate(item),
        }
    }

    fn modify(&self, f: impl FnOnce(&mut Vec<T>)) {
        let mut items = self.0.items.borrow_mut();
        f(Rc::make_mut(&mut items));
    }

    fn notify(&self, e: NotifyCollectionChangedEventArgs<'_, T>) {
        if !self.0.collection_changed.is_empty() {
            for (_, handler) in self.0.collection_changed.snapshot().iter() {
                handler(&e);
            }
        }
        if self.0.untyped_collection_changed.has_handlers() {
            self.0.untyped_collection_changed.raise(&CollectionChange {
                action: e.action,
                new_starting_index: e.new_starting_index,
                new_count: e.new_items.len(),
                old_starting_index: e.old_starting_index,
                old_count: e.old_items.len(),
            });
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

        self.notify_count_changed();
    }

    fn notify_count_changed(&self) {
        self.0.property_changed.raise("Count");
    }

    fn notify_remove(&self, items: &[T], index: usize) {
        self.notify(NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Remove,
            new_items: &[],
            old_items: items,
            new_starting_index: -1,
            old_starting_index: index as i32,
        });

        self.notify_count_changed();
    }
}

impl<T> INotifyCollectionChanged for FerroList<T> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.0.untyped_collection_changed
    }
}

impl<T> INotifyPropertyChanged for FerroList<T> {
    fn property_changed(&self) -> &Event<str> {
        &self.0.property_changed
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
    ///
    /// Each run of adjacent items that are removed is notified as one
    /// removal, from the end of the list to its start.
    pub fn remove_all(&self, items: impl IntoIterator<Item = T>) {
        let h_items: Vec<T> = items.into_iter().collect();
        let mut counter = 0;

        let mut i = self.count();
        while i > 0 {
            i -= 1;
            let contained = h_items.contains(&self.0.items.borrow()[i]);
            if contained {
                counter += 1;
            } else if counter > 0 {
                self.remove_range(i + 1, counter);
                counter = 0;
            }
        }

        if counter > 0 {
            self.remove_range(0, counter);
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

    type Logged = (NotifyCollectionChangedAction, Vec<i32>, Vec<i32>, i32, i32);

    /// Upstream `AssertEvent`: the changes `action` raises.
    fn assert_event(items: &FerroList<i32>, action: impl FnOnce(), expected_events: &[Logged]) {
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        let token = items.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, i32>| {
            l.borrow_mut().push((e.action, e.new_items.to_vec(), e.old_items.to_vec(), e.new_starting_index, e.old_starting_index));
        }));
        action();
        items.remove_collection_changed(token);
        assert_eq!(*log.borrow(), expected_events);
    }

    fn strings(items: impl IntoIterator<Item = String>) -> FerroList<String> {
        FerroList::from_items(items)
    }

    fn removals(list: &FerroList<String>) -> Rc<RefCell<Vec<(NotifyCollectionChangedAction, i32, Vec<String>)>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, String>| {
            l.borrow_mut().push((e.action, e.old_starting_index, e.old_items.to_vec()));
        }));
        log
    }

    #[test]
    fn items_passed_to_constructor_should_appear_in_list() {
        let items = [1, 2, 3];
        let target = FerroList::from_items(items);

        assert_eq!(target.to_vec(), items);
    }

    #[test]
    #[should_panic]
    fn insert_range_past_end_should_throw_exception() {
        let target = FerroList::<i32>::new();

        target.insert_range(1, vec![1]);
    }

    #[test]
    fn move_should_move_one_item() {
        let target = FerroList::from_items([1, 2, 3]);

        assert_event(&target, || target.move_item(0, 1), &[(NotifyCollectionChangedAction::Move, vec![1], vec![1], 1, 0)]);

        assert_eq!(target.to_vec(), [2, 1, 3]);
    }

    #[test]
    fn move_should_update_collection() {
        let target = FerroList::from_items([1, 2, 3]);

        target.move_item(2, 0);

        assert_eq!(target.to_vec(), [3, 1, 2]);
    }

    #[test]
    fn move_range_should_update_collection() {
        let target = FerroList::from_items([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        target.move_range(4, 3, 0);

        assert_eq!(target.to_vec(), [5, 6, 7, 1, 2, 3, 4, 8, 9, 10]);
    }

    #[test]
    fn move_range_can_move_to_end() {
        let target = FerroList::from_items([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        target.move_range(0, 5, 9);

        assert_eq!(target.to_vec(), [6, 7, 8, 9, 10, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn move_range_raises_correct_collection_changed_event() {
        let target = FerroList::from_items([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let moved = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_event(
            &target,
            || target.move_range(0, 9, 9),
            &[(NotifyCollectionChangedAction::Move, moved.clone(), moved, 9, 0)],
        );

        assert_eq!(target.to_vec(), [10, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn move_range_should_move_one_item() {
        let target = FerroList::from_items([1, 2, 3]);

        assert_event(&target, || target.move_range(0, 1, 1), &[(NotifyCollectionChangedAction::Move, vec![1], vec![1], 1, 0)]);

        assert_eq!(target.to_vec(), [2, 1, 3]);
    }

    #[test]
    fn adding_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);

        target.add(3);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Add, vec![3], vec![], 2, -1)]);
    }

    #[test]
    fn adding_items_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);

        target.add_range([3, 4]);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Add, vec![3, 4], vec![], 2, -1)]);
    }

    #[test]
    fn add_range_i_enumerable_should_raise_count_property_changed() {
        let target = FerroList::from_items([1, 2, 3, 4, 5]);
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        let t = target.clone();

        target.property_changed().add(Rc::new(move |e: &str| {
            assert_eq!(e, "Count");
            assert_eq!(t.count(), 7);
            r.set(true);
        }));

        target.add_range(6..8);

        assert!(raised.get());
    }

    #[test]
    fn add_range_items_should_raise_correct_collection_changed() {
        let target = FerroList::<Rc<i32>>::new();

        let event_items = Rc::new(RefCell::new(Vec::new()));
        let e = event_items.clone();

        target.add_collection_changed(Rc::new(move |args: &NotifyCollectionChangedEventArgs<'_, Rc<i32>>| {
            e.borrow_mut().extend(args.new_items.iter().cloned());
        }));

        target.add_range((0..10).map(Rc::new));

        let event_items = event_items.borrow();
        let target = target.to_vec();
        assert_eq!(event_items.len(), target.len());
        assert!(event_items.iter().zip(&target).all(|(a, b)| Rc::ptr_eq(a, b)));
    }

    #[test]
    fn replacing_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);

        target.set(1, 3);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Replace, vec![3], vec![2], 1, 1)]);
    }

    #[test]
    fn inserting_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);

        target.insert(1, 3);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Add, vec![3], vec![], 1, -1)]);
    }

    #[test]
    fn inserting_items_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2]);
        let log = tracked(&target);

        target.insert_range(1, [3, 4]);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Add, vec![3, 4], vec![], 1, -1)]);
    }

    #[test]
    fn removing_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2, 3]);
        let log = tracked(&target);

        target.remove(&3);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Remove, vec![], vec![3], -1, 2)]);
    }

    #[test]
    fn moving_item_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2, 3]);
        let log = tracked(&target);

        target.move_item(2, 0);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Move, vec![3], vec![3], 0, 2)]);
    }

    #[test]
    fn moving_items_should_raise_collection_changed() {
        let target = FerroList::from_items([1, 2, 3]);
        let log = tracked(&target);

        target.move_range(1, 2, 0);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Move, vec![2, 3], vec![2, 3], 0, 1)]);
    }

    #[test]
    fn clearing_items_should_raise_collection_changed_reset() {
        let target = FerroList::from_items([1, 2, 3]);
        let log = tracked(&target);

        target.clear();

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Reset, vec![], vec![], -1, -1)]);
    }

    #[test]
    fn clearing_items_should_raise_collection_changed_remove() {
        let target = FerroList::from_items([1, 2, 3]);
        target.set_reset_behavior(ResetBehavior::Remove);
        let log = tracked(&target);

        target.clear();

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Remove, vec![], vec![1, 2, 3], -1, 0)]);
    }

    #[test]
    fn can_copy_to_array_of_same_type() {
        let target = FerroList::from_items(["foo", "bar", "baz"].map(String::from));
        let mut result = vec![String::new(); 3];

        target.copy_to(&mut result, 0);

        assert_eq!(target.to_vec(), result);
    }

    #[test]
    fn remove_all_should_send_single_notification_for_sequential_range() {
        let target = strings((0..10).map(|x| format!("Item {x}")));
        let to_remove = ["Item 5", "Item 6", "Item 7"].map(String::from);
        let log = removals(&target);

        target.remove_all(to_remove.clone());

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Remove, 5, to_remove.to_vec())]);
    }

    #[test]
    fn remove_all_should_send_single_notification_for_sequential_range_with_duplicate_source_items() {
        let target = strings((0..20).map(|x| format!("Item {}", x / 2)));
        let to_remove = ["Item 5", "Item 6", "Item 7"].map(String::from);
        let log = removals(&target);

        target.remove_all(to_remove);

        let expected = ["Item 5", "Item 5", "Item 6", "Item 6", "Item 7", "Item 7"].map(String::from).to_vec();
        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Remove, 10, expected)]);
    }

    #[test]
    fn remove_all_should_send_multiple_notifications_for_non_sequential_range() {
        let target = strings((0..10).map(|x| format!("Item {x}")));
        let to_remove = [["Item 2", "Item 3"].map(String::from), ["Item 5", "Item 6"].map(String::from)];
        let log = removals(&target);

        target.remove_all(to_remove.concat());

        assert_eq!(
            *log.borrow(),
            vec![
                (NotifyCollectionChangedAction::Remove, 5, to_remove[1].to_vec()),
                (NotifyCollectionChangedAction::Remove, 2, to_remove[0].to_vec()),
            ]
        );
    }

    #[test]
    fn remove_all_should_send_multiple_notifications_for_sequential_range_with_nonsequential_duplicate_source_items() {
        let items: Vec<String> = (0..10).map(|x| format!("Item {x}")).collect();
        let target = strings(items.iter().chain(&items).cloned());
        let to_remove = ["Item 5", "Item 6", "Item 7"].map(String::from);
        let log = removals(&target);

        target.remove_all(to_remove.clone());

        assert_eq!(
            *log.borrow(),
            vec![
                (NotifyCollectionChangedAction::Remove, 15, to_remove.to_vec()),
                (NotifyCollectionChangedAction::Remove, 5, to_remove.to_vec()),
            ]
        );
    }

    #[test]
    fn remove_all_should_not_send_notification_for_items_not_present() {
        let target = strings((0..10).map(|x| format!("Item {x}")));
        let to_remove = ["Item 5", "Item 6", "Item 7", "Not present"].map(String::from);
        let log = removals(&target);

        target.remove_all(to_remove.clone());

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Remove, 5, to_remove[..3].to_vec())]);
    }

    #[test]
    fn remove_all_should_handle_empty_list() {
        let target = FerroList::<String>::new();
        let to_remove = ["Item 5", "Item 6", "Item 7"].map(String::from);
        let log = removals(&target);

        target.remove_all(to_remove);

        assert!(log.borrow().is_empty());
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
