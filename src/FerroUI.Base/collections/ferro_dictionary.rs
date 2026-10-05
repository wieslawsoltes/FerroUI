use super::{CollectionChangedHandler, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::data::core::INDEXER_NAME;
use crate::data::model::{CollectionChange, Event, INotifyCollectionChanged, INotifyPropertyChanged};
use crate::utilities::{HandlerList, WeakEventSender};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::rc::{Rc, Weak};

/// The storage of a [`FerroDictionary`]: a hash index over entry slots.
///
/// Enumeration follows the slots, and a removed slot is reused by the next
/// addition (most recently freed first), which gives the enumeration order of
/// the dictionary this mirrors: insertion order as long as nothing is
/// removed.
struct Inner<K, V> {
    entries: Vec<Option<(K, V)>>,
    free: Vec<usize>,
    map: HashMap<K, usize>,
}

impl<K: Eq + Hash + Clone, V: Clone> Inner<K, V> {
    fn with_capacity(capacity: usize) -> Self {
        Self { entries: Vec::with_capacity(capacity), free: Vec::new(), map: HashMap::with_capacity(capacity) }
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).and_then(|&slot| self.entries[slot].as_ref()).map(|(_, v)| v)
    }

    /// Inserts or replaces; returns the replaced value.
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&slot) = self.map.get(&key) {
            return self.entries[slot].replace((key, value)).map(|(_, old)| old);
        }
        let slot = match self.free.pop() {
            Some(slot) => {
                self.entries[slot] = Some((key.clone(), value));
                slot
            }
            None => {
                self.entries.push(Some((key.clone(), value)));
                self.entries.len() - 1
            }
        };
        self.map.insert(key, slot);
        None
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        let slot = self.map.remove(key)?;
        self.free.push(slot);
        self.entries[slot].take().map(|(_, v)| v)
    }

    fn to_vec(&self) -> Vec<(K, V)> {
        self.entries.iter().flatten().cloned().collect()
    }
}

/// A notifying dictionary.
///
/// Changes raise the collection changed event and property changed
/// notifications for `Count` and `Item[key]` (`Item` when the dictionary is
/// cleared). Keys are shown in those names through their [`Display`] form.
///
/// Enumeration works on a copy, so the dictionary may be modified from
/// within an enumeration or a change notification.
///
/// The dictionary is a reference object: the value is a shared handle,
/// clones refer to the same dictionary and handles compare by identity.
pub struct FerroDictionary<K, V>(Rc<FerroDictionaryData<K, V>>);

struct FerroDictionaryData<K, V> {
    inner: RefCell<Inner<K, V>>,
    collection_changed: HandlerList<CollectionChangedHandler<(K, V)>>,
    /// The untyped form of the collection changed event
    /// ([`INotifyCollectionChanged`]), raised after the typed handlers.
    untyped_collection_changed: Event<CollectionChange>,
    property_changed: Event<str>,
}

/// The weak form of a [`FerroDictionary`] handle.
pub struct WeakFerroDictionary<K, V>(Weak<FerroDictionaryData<K, V>>);

impl<K, V> Clone for WeakFerroDictionary<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: 'static, V: 'static> WeakEventSender for FerroDictionary<K, V> {
    type Weak = WeakFerroDictionary<K, V>;

    fn downgrade_sender(&self) -> WeakFerroDictionary<K, V> {
        WeakFerroDictionary(Rc::downgrade(&self.0))
    }

    fn upgrade_sender(weak: &WeakFerroDictionary<K, V>) -> Option<Self> {
        weak.0.upgrade().map(FerroDictionary)
    }

    fn sender_address(&self) -> usize {
        Rc::as_ptr(&self.0) as *const () as usize
    }
}

impl<K, V> Clone for FerroDictionary<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K, V> PartialEq for FerroDictionary<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<K, V> FerroDictionary<K, V> {
    /// Whether two handles refer to the same dictionary.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<K, V> std::fmt::Debug for FerroDictionary<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FerroDictionary")
    }
}

impl<K: Eq + Hash + Clone + Display, V: Clone> Default for FerroDictionary<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Clone + Display, V: Clone> FerroDictionary<K, V> {
    /// Creates an empty dictionary.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates an empty dictionary with room for `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Rc::new(FerroDictionaryData {
            inner: RefCell::new(Inner::with_capacity(capacity)),
            collection_changed: HandlerList::new(),
            untyped_collection_changed: Event::new(),
            property_changed: Event::new(),
        }))
    }

    /// Creates a dictionary with the entries of `dictionary`. A key that
    /// occurs more than once keeps its last value.
    pub fn from_dictionary(dictionary: impl IntoIterator<Item = (K, V)>) -> Self {
        let this = Self::new();
        {
            let mut inner = this.0.inner.borrow_mut();
            for (key, value) in dictionary {
                inner.insert(key, value);
            }
        }
        this
    }

    /// Subscribes to changes of the collection. Returns a token for
    /// [`remove_collection_changed`](Self::remove_collection_changed).
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<(K, V)>>) -> u64 {
        self.0.collection_changed.add(handler)
    }

    /// Unsubscribes a handler.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.collection_changed.remove(token)
    }

    /// Whether anything is subscribed to changes of the collection, typed or
    /// untyped.
    pub fn has_collection_changed_subscribers(&self) -> bool {
        !self.0.collection_changed.is_empty() || self.0.untyped_collection_changed.has_handlers()
    }

    /// The number of entries.
    pub fn count(&self) -> usize {
        self.0.inner.borrow().map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn is_read_only(&self) -> bool {
        false
    }

    /// The keys, in enumeration order.
    pub fn keys(&self) -> Vec<K> {
        self.0.inner.borrow().entries.iter().flatten().map(|(k, _)| k.clone()).collect()
    }

    /// The values, in enumeration order.
    pub fn values(&self) -> Vec<V> {
        self.0.inner.borrow().entries.iter().flatten().map(|(_, v)| v.clone()).collect()
    }

    /// The value for `key`. Panics if the key is not present.
    pub fn get(&self, key: &K) -> V {
        match self.try_get_value(key) {
            Some(value) => value,
            None => panic!("The given key '{key}' was not present in the dictionary."),
        }
    }

    /// The value for `key`, if present.
    pub fn try_get_value(&self, key: &K) -> Option<V> {
        self.0.inner.borrow().get(key).cloned()
    }

    /// Sets the value for `key`, adding the entry if it is not present.
    pub fn set(&self, key: K, value: V) {
        let old = self.0.inner.borrow_mut().insert(key.clone(), value.clone());
        match old {
            Some(old) => {
                self.0.property_changed.raise(&format!("{INDEXER_NAME}[{key}]"));
                if self.has_collection_changed_subscribers() {
                    self.raise_collection_changed(
                        NotifyCollectionChangedAction::Replace,
                        &[(key.clone(), value)],
                        &[(key, old)],
                    );
                }
            }
            None => self.notify_add(key, value),
        }
    }

    /// Adds an entry. Panics if the key is already present.
    pub fn add(&self, key: K, value: V) {
        {
            let mut inner = self.0.inner.borrow_mut();
            if inner.map.contains_key(&key) {
                drop(inner);
                panic!("An item with the same key has already been added. Key: {key}");
            }
            inner.insert(key.clone(), value.clone());
        }
        self.notify_add(key, value);
    }

    /// Removes all entries.
    pub fn clear(&self) {
        let old = std::mem::replace(&mut *self.0.inner.borrow_mut(), Inner::with_capacity(0));

        self.0.property_changed.raise("Count");
        self.0.property_changed.raise(INDEXER_NAME);

        if self.has_collection_changed_subscribers() {
            self.raise_collection_changed(NotifyCollectionChangedAction::Remove, &[], &old.to_vec());
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.0.inner.borrow().map.contains_key(key)
    }

    /// Whether the dictionary has the entry `key` with a value equal to
    /// `value`.
    pub fn contains(&self, key: &K, value: &V) -> bool
    where
        V: PartialEq,
    {
        self.0.inner.borrow().get(key).is_some_and(|v| v == value)
    }

    /// The entries, in enumeration order.
    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.0.inner.borrow().to_vec()
    }

    /// Calls `f` for each entry of a copy of the dictionary.
    pub fn for_each(&self, mut f: impl FnMut(&K, &V)) {
        for (key, value) in self.to_vec() {
            f(&key, &value);
        }
    }

    /// Removes the entry for `key`. Returns false if it is not present.
    pub fn remove(&self, key: &K) -> bool {
        let removed = self.0.inner.borrow_mut().remove(key);
        match removed {
            Some(value) => {
                self.0.property_changed.raise("Count");
                self.0.property_changed.raise(&format!("{INDEXER_NAME}[{key}]"));

                if self.has_collection_changed_subscribers() {
                    self.raise_collection_changed(NotifyCollectionChangedAction::Remove, &[], &[(key.clone(), value)]);
                }
                true
            }
            None => false,
        }
    }

    fn notify_add(&self, key: K, value: V) {
        self.0.property_changed.raise("Count");
        self.0.property_changed.raise(&format!("{INDEXER_NAME}[{key}]"));

        if self.has_collection_changed_subscribers() {
            self.raise_collection_changed(NotifyCollectionChangedAction::Add, &[(key, value)], &[]);
        }
    }

    fn raise_collection_changed(&self, action: NotifyCollectionChangedAction, new_items: &[(K, V)], old_items: &[(K, V)]) {
        // Entries of a dictionary have no position.
        let e = NotifyCollectionChangedEventArgs { action, new_items, old_items, new_starting_index: -1, old_starting_index: -1 };
        for (_, handler) in self.0.collection_changed.snapshot().iter() {
            handler(&e);
        }
        self.0.untyped_collection_changed.raise(&CollectionChange {
            action,
            new_starting_index: -1,
            new_count: new_items.len(),
            old_starting_index: -1,
            old_count: old_items.len(),
        });
    }
}

impl<K, V> INotifyCollectionChanged for FerroDictionary<K, V> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.0.untyped_collection_changed
    }
}

impl<K: Eq + Hash + Clone + Display, V: Clone> INotifyPropertyChanged for FerroDictionary<K, V> {
    fn property_changed(&self) -> &Event<str> {
        &self.0.property_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    type Args = (NotifyCollectionChangedAction, Vec<(String, String)>, Vec<(String, String)>, i32, i32);

    struct CollectionChangedTracker {
        args: Rc<RefCell<Option<Args>>>,
    }

    impl CollectionChangedTracker {
        fn new(target: &FerroDictionary<String, String>) -> Self {
            let args = Rc::new(RefCell::new(None));
            let sink = args.clone();
            target.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, (String, String)>| {
                *sink.borrow_mut() =
                    Some((e.action, e.new_items.to_vec(), e.old_items.to_vec(), e.new_starting_index, e.old_starting_index));
            }));
            Self { args }
        }

        fn args(&self) -> Args {
            self.args.borrow().clone().expect("a collection changed notification")
        }
    }

    struct PropertyChangedTracker {
        names: Rc<RefCell<Vec<String>>>,
    }

    impl PropertyChangedTracker {
        fn new(target: &FerroDictionary<String, String>) -> Self {
            let names = Rc::new(RefCell::new(Vec::new()));
            let sink = names.clone();
            target.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
            Self { names }
        }

        fn names(&self) -> Vec<String> {
            self.names.borrow().clone()
        }
    }

    fn s(value: &str) -> String {
        value.to_string()
    }

    fn pair(key: &str, value: &str) -> (String, String) {
        (s(key), s(value))
    }

    #[test]
    fn adding_item_should_raise_collection_changed() {
        let target = FerroDictionary::<String, String>::new();
        let tracker = CollectionChangedTracker::new(&target);

        target.add(s("foo"), s("bar"));

        let (action, new_items, _, new_starting_index, _) = tracker.args();
        assert_eq!(NotifyCollectionChangedAction::Add, action);
        assert_eq!(-1, new_starting_index);
        assert_eq!(1, new_items.len());
        assert_eq!(pair("foo", "bar"), new_items[0]);
    }

    #[test]
    fn adding_item_should_raise_property_changed() {
        let target = FerroDictionary::<String, String>::new();
        let tracker = PropertyChangedTracker::new(&target);

        target.add(s("foo"), s("bar"));

        assert_eq!(vec!["Count", "Item[foo]"], tracker.names());
    }

    #[test]
    fn assigning_item_should_raise_collection_changed_add() {
        let target = FerroDictionary::<String, String>::new();
        let tracker = CollectionChangedTracker::new(&target);

        target.set(s("foo"), s("bar"));

        let (action, new_items, _, new_starting_index, _) = tracker.args();
        assert_eq!(NotifyCollectionChangedAction::Add, action);
        assert_eq!(-1, new_starting_index);
        assert_eq!(1, new_items.len());
        assert_eq!(pair("foo", "bar"), new_items[0]);
    }

    #[test]
    fn assigning_item_should_raise_collection_changed_replace() {
        let target = FerroDictionary::<String, String>::new();

        target.set(s("foo"), s("baz"));
        let tracker = CollectionChangedTracker::new(&target);
        target.set(s("foo"), s("bar"));

        let (action, new_items, _, new_starting_index, _) = tracker.args();
        assert_eq!(NotifyCollectionChangedAction::Replace, action);
        assert_eq!(-1, new_starting_index);
        assert_eq!(1, new_items.len());
        assert_eq!(pair("foo", "bar"), new_items[0]);
    }

    #[test]
    fn assigning_item_should_raise_property_changed_add() {
        let target = FerroDictionary::<String, String>::new();
        let tracker = PropertyChangedTracker::new(&target);

        target.set(s("foo"), s("bar"));

        assert_eq!(vec!["Count", "Item[foo]"], tracker.names());
    }

    #[test]
    fn assigning_item_should_raise_property_changed_replace() {
        let target = FerroDictionary::<String, String>::new();

        target.set(s("foo"), s("baz"));
        let tracker = PropertyChangedTracker::new(&target);
        target.set(s("foo"), s("bar"));

        assert_eq!(vec!["Item[foo]"], tracker.names());
    }

    #[test]
    fn removing_item_should_raise_collection_changed() {
        let target = FerroDictionary::<String, String>::new();

        target.set(s("foo"), s("bar"));
        let tracker = CollectionChangedTracker::new(&target);
        target.remove(&s("foo"));

        let (action, _, old_items, _, old_starting_index) = tracker.args();
        assert_eq!(NotifyCollectionChangedAction::Remove, action);
        assert_eq!(-1, old_starting_index);
        assert_eq!(1, old_items.len());
        assert_eq!(pair("foo", "bar"), old_items[0]);
    }

    #[test]
    fn remove_method_should_remove_item_from_collection() {
        let target = FerroDictionary::from_dictionary([pair("foo", "bar")]);
        assert_eq!(target.count(), 1);

        target.remove(&s("foo"));
        assert_eq!(target.count(), 0);
    }

    #[test]
    fn removing_item_should_raise_property_changed() {
        let target = FerroDictionary::<String, String>::new();

        target.set(s("foo"), s("bar"));
        let tracker = PropertyChangedTracker::new(&target);
        target.remove(&s("foo"));

        assert_eq!(vec!["Count", "Item[foo]"], tracker.names());
    }

    #[test]
    fn clearing_collection_should_raise_collection_changed() {
        let target = FerroDictionary::<String, String>::new();

        target.set(s("foo"), s("bar"));
        target.set(s("baz"), s("qux"));
        let tracker = CollectionChangedTracker::new(&target);
        target.clear();

        let (action, _, old_items, _, old_starting_index) = tracker.args();
        assert_eq!(NotifyCollectionChangedAction::Remove, action);
        assert_eq!(-1, old_starting_index);
        assert_eq!(2, old_items.len());
        assert_eq!(pair("foo", "bar"), old_items[0]);
    }

    #[test]
    fn clearing_collection_should_raise_property_changed() {
        let target = FerroDictionary::<String, String>::new();

        target.set(s("foo"), s("bar"));
        target.set(s("baz"), s("qux"));
        let tracker = PropertyChangedTracker::new(&target);
        target.clear();

        assert_eq!(vec!["Count", INDEXER_NAME], tracker.names());
    }

    #[test]
    fn constructor_should_initialize_with_provided_collection() {
        let initial_collection = HashMap::from([pair("key1", "value1"), pair("key2", "value2")]);

        let target = FerroDictionary::from_dictionary(initial_collection);

        assert_eq!(2, target.count());
        assert_eq!("value1", target.get(&s("key1")));
        assert_eq!("value2", target.get(&s("key2")));
    }

    // Not upstream: the enumeration order the notifications above rely on.
    #[test]
    fn enumeration_follows_insertion_and_reuses_removed_slots() {
        let target = FerroDictionary::<String, i32>::new();
        target.add(s("a"), 1);
        target.add(s("b"), 2);
        target.add(s("c"), 3);
        assert_eq!(vec![s("a"), s("b"), s("c")], target.keys());

        target.remove(&s("b"));
        target.add(s("d"), 4);
        assert_eq!(vec![s("a"), s("d"), s("c")], target.keys());
        assert_eq!(vec![1, 4, 3], target.values());
        assert!(target.contains_key(&s("d")));
        assert!(!target.contains_key(&s("b")));
        assert!(target.contains(&s("d"), &4));
        assert_eq!(None, target.try_get_value(&s("b")));
    }
}
