use crate::items_source::{ItemsChangedEventArgs, ItemsSource};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// Receives the changes of a collection in three phases; every listener of
/// a collection receives a phase before any listener receives the next one.
pub trait ICollectionChangedListener {
    fn pre_changed(&self, e: &ItemsChangedEventArgs<'_>);
    fn changed(&self, e: &ItemsChangedEventArgs<'_>);
    fn post_changed(&self, e: &ItemsChangedEventArgs<'_>);
}

struct Entry {
    id: u64,
    token: u64,
    listeners: RefCell<Vec<Weak<dyn ICollectionChangedListener>>>,
}

impl Entry {
    fn notify(&self, e: &ItemsChangedEventArgs<'_>) {
        // Listeners may be added and removed while notifying.
        let listeners: Vec<Rc<dyn ICollectionChangedListener>> =
            self.listeners.borrow().iter().filter_map(Weak::upgrade).collect();

        for l in &listeners {
            l.pre_changed(e);
        }

        for l in &listeners {
            l.changed(e);
        }

        for l in &listeners {
            l.post_changed(e);
        }
    }
}

/// Removes the entry of a collection when the collection (and with it the
/// handler subscribed to it) is released.
struct EntryGuard {
    key: usize,
    id: u64,
}

impl Drop for EntryGuard {
    fn drop(&mut self) {
        let (key, id) = (self.key, self.id);
        let _ = ENTRIES.try_with(|entries| {
            if let Ok(mut entries) = entries.try_borrow_mut() {
                // The slot may already belong to a newer subscription.
                if entries.get(&key).is_some_and(|entry| entry.id == id) {
                    entries.remove(&key);
                }
            }
        });
    }
}

thread_local! {
    static ENTRIES: RefCell<HashMap<usize, Rc<Entry>>> = RefCell::new(HashMap::new());
    static NEXT_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}

fn listener_id(listener: &Weak<dyn ICollectionChangedListener>) -> usize {
    listener.as_ptr() as *const () as usize
}

/// Dispatches the changes of notifying collections to weakly held
/// listeners. A collection is subscribed to once, however many listeners it
/// has.
pub struct CollectionChangedEventManager;

impl CollectionChangedEventManager {
    /// Adds a listener to a collection. Panics if the listener is already
    /// registered for the collection. Does nothing for a collection that
    /// does not notify.
    pub fn add_listener(collection: &ItemsSource, listener: Weak<dyn ICollectionChangedListener>) {
        if !collection.is_notifying() {
            return;
        }

        let key = collection.identity();
        let entry = ENTRIES.with(|entries| entries.borrow().get(&key).cloned());

        let entry = match entry {
            Some(entry) => entry,
            None => {
                let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
                let guard = EntryGuard { key, id };
                let token = collection
                    .list()
                    .add_collection_changed(Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
                        // Capture the guard as a whole: it lives as long as
                        // the handler does.
                        let guard = &guard;
                        let entry = ENTRIES.with(|entries| entries.borrow().get(&guard.key).cloned());
                        if let Some(entry) = entry.filter(|entry| entry.id == guard.id) {
                            entry.notify(e);
                        }
                    }))
                    .expect("a notifying collection accepts handlers");
                let entry = Rc::new(Entry { id, token, listeners: RefCell::new(Vec::new()) });
                ENTRIES.with(|entries| entries.borrow_mut().insert(key, entry.clone()));
                entry
            }
        };

        let id = listener_id(&listener);
        let mut listeners = entry.listeners.borrow_mut();
        listeners.retain(|l| l.strong_count() > 0);

        if listeners.iter().any(|l| listener_id(l) == id) {
            panic!("Collection listener already added for this collection/listener combination.");
        }

        listeners.push(listener);
    }

    /// Removes a listener from a collection. Panics if the listener is not
    /// registered for the collection.
    pub fn remove_listener(collection: &ItemsSource, listener: &Weak<dyn ICollectionChangedListener>) {
        if !collection.is_notifying() {
            return;
        }

        let key = collection.identity();
        let entry = ENTRIES.with(|entries| entries.borrow().get(&key).cloned());

        if let Some(entry) = entry {
            let id = listener_id(listener);
            let mut listeners = entry.listeners.borrow_mut();
            let position = listeners.iter().position(|l| listener_id(l) == id);

            if let Some(position) = position {
                listeners.remove(position);
                listeners.retain(|l| l.strong_count() > 0);

                if listeners.is_empty() {
                    drop(listeners);
                    // Unsubscribing releases the handler and with it the
                    // guard that removes the entry.
                    ENTRIES.with(|entries| entries.borrow_mut().remove(&key));
                    collection.list().remove_collection_changed(entry.token);
                }

                return;
            }
        }

        panic!("Collection listener not registered for this collection/listener combination.");
    }
}
