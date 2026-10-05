use super::{CollectionChangedEventManager, ICollectionChangedListener};
use crate::items_source::{IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

fn listener_of(listener: &Rc<Listener>) -> Weak<dyn ICollectionChangedListener> {
    let weak: Weak<Listener> = Rc::downgrade(listener);
    weak
}

#[test]
fn add_listener_listens_to_events() {
    let source: Rc<FerroList<String>> = Rc::new(FerroList::new());
    let listener = Rc::new(Listener::default());

    CollectionChangedEventManager::add_listener(&source.clone().into(), listener_of(&listener));

    assert!(listener.received.borrow().is_empty());

    source.add("foo".to_string());

    assert_eq!(listener.received.borrow().len(), 1);
}

#[test]
fn remove_listener_stops_listening_to_events() {
    let source: Rc<FerroList<String>> = Rc::new(FerroList::new());
    let listener = Rc::new(Listener::default());

    CollectionChangedEventManager::add_listener(&source.clone().into(), listener_of(&listener));
    CollectionChangedEventManager::remove_listener(&source.clone().into(), &listener_of(&listener));

    source.add("foo".to_string());

    assert!(listener.received.borrow().is_empty());
}

#[test]
fn receives_events_from_wrapped_collection() {
    let source = Rc::new(WrappingCollection::default());
    let listener = Rc::new(Listener::default());

    CollectionChangedEventManager::add_listener(&ItemsSource::new(source.clone()), listener_of(&listener));

    assert!(listener.received.borrow().is_empty());

    source.add("foo");

    assert_eq!(listener.received.borrow().len(), 1);
}

#[derive(Default)]
struct Listener {
    received: RefCell<Vec<NotifyCollectionChangedAction>>,
}

impl ICollectionChangedListener for Listener {
    fn changed(&self, e: &ItemsChangedEventArgs<'_>) {
        self.received.borrow_mut().push(e.action);
    }

    fn post_changed(&self, _e: &ItemsChangedEventArgs<'_>) {}

    fn pre_changed(&self, _e: &ItemsChangedEventArgs<'_>) {}
}

/// A notifying collection that forwards the subscriptions to the list it
/// wraps.
#[derive(Default)]
struct WrappingCollection {
    inner: FerroList<String>,
}

impl WrappingCollection {
    fn add(&self, s: &str) {
        self.inner.add(s.to_string());
    }
}

impl IItemsList for WrappingCollection {
    fn count(&self) -> usize {
        IItemsList::count(&self.inner)
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        IItemsList::get_at(&self.inner, index)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        IItemsList::add_collection_changed(&self.inner, handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        IItemsList::remove_collection_changed(&self.inner, token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
