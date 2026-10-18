//! `Utils/CollectionChangedEventManager.cs`: weak listeners of the changes of a collection of
//! models, notified in three phases (all `pre_changed`, then all `changed`, then all
//! `post_changed`), so that the selection can prepare for a change before the rows apply it and
//! settle after.

use crate::model_collection::{collection_identity, Models, ModelsChangedArgs};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use rustc_hash::FxHashMap;
use std::rc::{Rc, Weak};

/// A listener of [`CollectionChangedEventManager`].
pub trait ICollectionChangedListener<M: 'static> {
    fn pre_changed(&self, _collection: &Models<M>, _e: &ModelsChangedArgs<'_, M>) {}
    fn changed(&self, collection: &Models<M>, e: &ModelsChangedArgs<'_, M>);
    fn post_changed(&self, _collection: &Models<M>, _e: &ModelsChangedArgs<'_, M>) {}
}

struct Entry<M: 'static> {
    collection: Weak<dyn crate::model_collection::ModelCollection<M>>,
    token: u64,
    listeners: Vec<Weak<dyn ICollectionChangedListener<M>>>,
}

thread_local! {
    static ENTRIES: RefCell<FxHashMap<(usize, TypeId), Box<dyn Any>>> = RefCell::new(FxHashMap::default());
}

/// The listeners of the collections of the thread.
pub struct CollectionChangedEventManager;

impl CollectionChangedEventManager {
    /// Adds `listener` to the listeners of `collection`; the first listener subscribes to it. A
    /// collection that does not notify gets no listener.
    pub fn add_listener<M: 'static>(collection: &Models<M>, listener: Weak<dyn ICollectionChangedListener<M>>) {
        if !collection.is_notifying() {
            return;
        }
        let key = (collection_identity(collection), TypeId::of::<M>());
        let added = ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            match entries.get_mut(&key).and_then(|e| e.downcast_mut::<Entry<M>>()) {
                // The entry of a collection that died, whose address a new collection took.
                Some(entry) if entry.collection.strong_count() == 0 => {
                    entries.remove(&key);
                    false
                }
                Some(entry) => {
                    entry.listeners.push(listener.clone());
                    true
                }
                None => false,
            }
        });
        if added {
            return;
        }
        let handler: Rc<crate::model_collection::ModelsChangedHandler<M>> = Rc::new(move |e: &ModelsChangedArgs<'_, M>| {
            Self::on_changed::<M>(key, e);
        });
        let token = collection.add_collection_changed(handler).unwrap_or(0);
        let entry = Entry::<M> { collection: Rc::downgrade(collection), token, listeners: vec![listener] };
        ENTRIES.with(|entries| entries.borrow_mut().insert(key, Box::new(entry)));
    }

    /// Removes `listener` from the listeners of `collection`; the last listener unsubscribes.
    ///
    /// # Panics
    /// When the listener is not registered (`InvalidOperationException`).
    pub fn remove_listener<M: 'static>(collection: &Models<M>, listener: &Weak<dyn ICollectionChangedListener<M>>) {
        if !collection.is_notifying() {
            return;
        }
        let key = (collection_identity(collection), TypeId::of::<M>());
        let removed = ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            let Some(entry) = entries.get_mut(&key).and_then(|e| e.downcast_mut::<Entry<M>>()) else {
                return None;
            };
            let position = entry.listeners.iter().position(|l| Weak::ptr_eq(l, listener))?;
            entry.listeners.remove(position);
            if entry.listeners.is_empty() {
                let token = entry.token;
                entries.remove(&key);
                Some(Some(token))
            } else {
                Some(None)
            }
        });
        match removed {
            None => panic!("Collection listener is not registered."),
            Some(Some(token)) => collection.remove_collection_changed(token),
            Some(None) => {}
        }
    }

    fn on_changed<M: 'static>(key: (usize, TypeId), e: &ModelsChangedArgs<'_, M>) {
        // A stable batch: subscriptions can change during the callbacks.
        let snapshot = ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            let entry = entries.get_mut(&key).and_then(|e| e.downcast_mut::<Entry<M>>())?;
            entry.listeners.retain(|l| l.strong_count() > 0);
            let listeners: Vec<Rc<dyn ICollectionChangedListener<M>>> =
                entry.listeners.iter().filter_map(|l| l.upgrade()).collect();
            Some((entry.collection.upgrade(), listeners))
        });
        let Some((Some(collection), listeners)) = snapshot else {
            return;
        };
        for listener in &listeners {
            listener.pre_changed(&collection, e);
        }
        for listener in &listeners {
            listener.changed(&collection, e);
        }
        for listener in &listeners {
            listener.post_changed(&collection, e);
        }
        drop(listeners);
        // The last listener died: the collection loses its subscription.
        let token = ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            let empty = entries
                .get_mut(&key)
                .and_then(|e| e.downcast_mut::<Entry<M>>())
                .map(|entry| {
                    entry.listeners.retain(|l| l.strong_count() > 0);
                    entry.listeners.is_empty()
                })
                .unwrap_or(false);
            if empty {
                entries.remove(&key).and_then(|e| e.downcast::<Entry<M>>().ok()).map(|e| e.token)
            } else {
                None
            }
        });
        if let Some(token) = token {
            collection.remove_collection_changed(token);
        }
    }
}
