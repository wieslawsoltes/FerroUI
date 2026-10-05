use crate::data::model::{CollectionChange, INotifyCollectionChanged};
use crate::reactive::{AnonymousObserver, Disposable, IDisposable, IObservable, IObserver};
use crate::utilities::{IWeakEventSubscriber, WeakEvents};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub struct NotifyCollectionChangedExtensions;

impl NotifyCollectionChangedExtensions {
    /// Gets a weak observable for the CollectionChanged event.
    pub fn get_weak_collection_changed_observable(
        collection: &Rc<dyn INotifyCollectionChanged>,
    ) -> Rc<dyn IObservable<CollectionChange>> {
        WeakCollectionChangedObservable::new(Rc::downgrade(collection))
    }

    /// Subscribes to the CollectionChanged event using a weak subscription.
    ///
    /// `handler` is called with the collection and the change when the
    /// collection event is raised. Disposing the returned handle terminates
    /// the subscription.
    pub fn weak_subscribe(
        collection: &Rc<dyn INotifyCollectionChanged>,
        handler: impl Fn(&Rc<dyn INotifyCollectionChanged>, &CollectionChange) + 'static,
    ) -> Rc<dyn IDisposable> {
        let sender = collection.clone();
        Self::get_weak_collection_changed_observable(collection)
            .subscribe(Rc::new(AnonymousObserver::new(move |e: CollectionChange| handler(&sender, &e))))
    }

    /// Subscribes to the CollectionChanged event using a weak subscription.
    ///
    /// `handler` is called with the change when the collection event is
    /// raised. Disposing the returned handle terminates the subscription.
    pub fn weak_subscribe_action(
        collection: &Rc<dyn INotifyCollectionChanged>,
        handler: impl Fn(&CollectionChange) + 'static,
    ) -> Rc<dyn IDisposable> {
        Self::get_weak_collection_changed_observable(collection)
            .subscribe(Rc::new(AnonymousObserver::new(move |e: CollectionChange| handler(&e))))
    }
}

/// The observable of [`NotifyCollectionChangedExtensions::get_weak_collection_changed_observable`]:
/// subscribed to the collection through [`WeakEvents::collection_changed`]
/// while it has observers.
///
/// Its observer list, and the subscription when the first observer arrives
/// and the unsubscription when the last one leaves, are those of the
/// lightweight observable base class of the original, which is not ported.
struct WeakCollectionChangedObservable {
    this: Weak<WeakCollectionChangedObservable>,
    source_reference: Weak<dyn INotifyCollectionChanged>,
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<CollectionChange>>)>>,
    next_id: Cell<u64>,
}

impl WeakCollectionChangedObservable {
    fn new(source: Weak<dyn INotifyCollectionChanged>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            source_reference: source,
            observers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
        })
    }

    fn as_subscriber(&self) -> Option<Rc<dyn IWeakEventSubscriber<CollectionChange>>> {
        self.this.upgrade().map(|this| this as Rc<dyn IWeakEventSubscriber<CollectionChange>>)
    }

    fn initialize(&self) {
        if let (Some(instance), Some(this)) = (self.source_reference.upgrade(), self.as_subscriber()) {
            WeakEvents::collection_changed().subscribe(&instance, &this);
        }
    }

    fn deinitialize(&self) {
        if let (Some(instance), Some(this)) = (self.source_reference.upgrade(), self.as_subscriber()) {
            WeakEvents::collection_changed().unsubscribe(&instance, &this);
        }
    }

    fn remove(&self, id: u64) {
        let mut observers = self.observers.borrow_mut();
        if let Some(index) = observers.iter().position(|(i, _)| *i == id) {
            observers.remove(index);

            if observers.is_empty() {
                observers.shrink_to_fit();
                drop(observers);
                self.deinitialize();
            }
        }
    }

    fn publish_next(&self, value: CollectionChange) {
        let observers: Vec<Rc<dyn IObserver<CollectionChange>>> =
            self.observers.borrow().iter().map(|(_, observer)| observer.clone()).collect();
        for observer in observers {
            observer.on_next(value);
        }
    }
}

impl IObservable<CollectionChange> for WeakCollectionChangedObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<CollectionChange>>) -> Rc<dyn IDisposable> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let first = {
            let mut observers = self.observers.borrow_mut();
            let first = observers.is_empty();
            observers.push((id, observer));
            first
        };

        if first {
            self.initialize();
        }

        // The handle holds the observable: the collection holds it weakly.
        let parent = self.this.upgrade();
        Disposable::create(move || {
            if let Some(parent) = parent {
                parent.remove(id);
            }
        })
    }
}

impl IWeakEventSubscriber<CollectionChange> for WeakCollectionChangedObservable {
    fn on_event(&self, _sender: Option<&dyn Any>, _ev: &dyn Any, e: &CollectionChange) {
        self.publish_next(*e);
    }
}

#[cfg(test)]
mod tests {
    //! Not from upstream: the managed test suite has no tests for these
    //! extensions.

    use super::*;
    use crate::collections::{FerroList, NotifyCollectionChangedAction};

    #[test]
    fn weak_subscribe_receives_changes_until_disposed() {
        let list = FerroList::from_items([1, 2]);
        let collection: Rc<dyn INotifyCollectionChanged> = Rc::new(list.clone());
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();

        let subscription = NotifyCollectionChangedExtensions::weak_subscribe_action(&collection, move |e| {
            l.borrow_mut().push((e.action, e.new_starting_index, e.new_count));
        });
        list.add(3);
        subscription.dispose();
        list.add(4);

        assert_eq!(*log.borrow(), vec![(NotifyCollectionChangedAction::Add, 2, 1)]);
        assert!(!list.has_collection_changed_subscribers());
    }

    #[test]
    fn weak_subscription_does_not_keep_the_handler_alive() {
        let list = FerroList::from_items([1, 2]);
        let collection: Rc<dyn INotifyCollectionChanged> = Rc::new(list.clone());
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();

        let subscription = NotifyCollectionChangedExtensions::weak_subscribe(&collection, move |_, _| c.set(c.get() + 1));
        list.add(3);
        drop(subscription);
        list.add(4);

        assert_eq!(calls.get(), 1);
        assert!(!list.has_collection_changed_subscribers());
    }
}
