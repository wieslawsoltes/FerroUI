use super::{CollectionChangedHandler, IFerroReadOnlyList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::data::core::ValueTypes;
use crate::data::model::ModelTypes;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::WeakEventSender;
use crate::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Defines extension methods for working with [`FerroList`](super::FerroList)s.
pub struct FerroListExtensions;

impl FerroListExtensions {
    /// Invokes an action for each item in a collection and subsequently each
    /// item added or removed from the collection.
    ///
    /// `added` is called initially for each item in the collection and
    /// subsequently for each item added to the collection; `removed` for
    /// each item removed from the collection; `reset` when the collection is
    /// reset. `weak_subscription` indicates if a weak subscription should be
    /// used to track changes to the collection. Returns a disposable used to
    /// terminate the subscription.
    pub fn for_each_item<T, L>(
        collection: &L,
        added: impl Fn(&T) + 'static,
        removed: impl Fn(&T) + 'static,
        reset: impl Fn() + 'static,
        weak_subscription: bool,
    ) -> Rc<dyn IDisposable>
    where
        T: Clone + 'static,
        L: IFerroReadOnlyList<T> + WeakEventSender,
    {
        Self::for_each_item_indexed(collection, move |_, i| added(i), move |_, i| removed(i), reset, weak_subscription)
    }

    /// Invokes an action for each item in a collection and subsequently each
    /// item added or removed from the collection.
    ///
    /// `added` is called initially for each item in the collection and
    /// subsequently for each item added to the collection, with the index in
    /// the collection and the item; `removed` for each item removed from the
    /// collection, with the index in the collection and the item; `reset`
    /// when the collection is reset, which is followed by calls to `added`
    /// for each item present in the collection after the reset.
    /// `weak_subscription` indicates if a weak subscription should be used to
    /// track changes to the collection. Returns a disposable used to
    /// terminate the subscription.
    ///
    /// A weak subscription is held by the returned disposable: the collection
    /// holds the handler weakly, as the weak collection changed subscription
    /// of the original does, and the handler goes when the disposable is
    /// disposed or dropped.
    pub fn for_each_item_indexed<T, L>(
        collection: &L,
        added: impl Fn(usize, &T) + 'static,
        removed: impl Fn(usize, &T) + 'static,
        reset: impl Fn() + 'static,
        weak_subscription: bool,
    ) -> Rc<dyn IDisposable>
    where
        T: Clone + 'static,
        L: IFerroReadOnlyList<T> + WeakEventSender,
    {
        let add = Rc::new(move |index: usize, items: &[T]| {
            let mut index = index;
            for item in items {
                added(index, item);
                index += 1;
            }
        });

        let remove = move |index: usize, items: &[T]| {
            for i in (0..items.len()).rev() {
                removed(index + i, &items[i]);
            }
        };

        // The handler belongs to the collection, so it reads the collection
        // through a weak reference.
        let source = collection.downgrade_sender();
        let add_items = add.clone();
        let handler: Rc<CollectionChangedHandler<T>> = Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, T>| {
            let reset_collection = || {
                reset();
                if let Some(collection) = L::upgrade_sender(&source) {
                    add_items(0, &collection.snapshot());
                }
            };

            match e.action {
                NotifyCollectionChangedAction::Add => add_items(e.new_starting_index as usize, e.new_items),

                NotifyCollectionChangedAction::Move => {
                    if e.old_starting_index < 0 {
                        return reset_collection();
                    }

                    remove(e.old_starting_index as usize, e.old_items);
                    let mut new_index = e.new_starting_index;

                    if new_index > e.old_starting_index {
                        new_index -= e.old_items.len() as i32 - 1;
                    }

                    add_items(new_index as usize, e.new_items);
                }
                NotifyCollectionChangedAction::Replace => {
                    if e.old_starting_index < 0 {
                        return reset_collection();
                    }

                    remove(e.old_starting_index as usize, e.old_items);
                    add_items(e.new_starting_index as usize, e.new_items);
                }

                NotifyCollectionChangedAction::Remove => remove(e.old_starting_index as usize, e.old_items),

                NotifyCollectionChangedAction::Reset => reset_collection(),
            }
        });

        add(0, &collection.snapshot());

        if weak_subscription {
            weak_subscribe(collection, handler, L::add_collection_changed, L::remove_collection_changed)
        } else {
            let token = collection.add_collection_changed(handler);

            let collection = collection.clone();
            Disposable::create(move || {
                collection.remove_collection_changed(token);
            })
        }
    }

    /// Listens for property changed events from all items in a collection.
    ///
    /// `callback` is called for each property changed event with the item
    /// that raised it and the name of the property. An item raises property
    /// changed events if its model type declares them
    /// ([`ModelTypes`]); an item that is a shared model object (`Rc<M>` of a
    /// registered model type `M`, or a boxed value) is the object itself.
    /// Returns a disposable used to terminate the subscription.
    ///
    /// # Panics
    /// The subscription panics when the collection is reset, as the original
    /// throws.
    pub fn track_item_property_changed<T, L>(
        collection: &L,
        callback: impl Fn((Option<BoxedValue>, &str)) + 'static,
    ) -> Rc<dyn IDisposable>
    where
        T: Clone + PartialEq + 'static,
        L: IFerroReadOnlyList<T> + WeakEventSender,
    {
        let tracked: Rc<RefCell<Vec<(BoxedValue, u64)>>> = Rc::new(RefCell::new(Vec::new()));
        let callback = Rc::new(callback);

        let handler = move |inpc: &BoxedValue| -> Rc<dyn Fn(&str)> {
            let sender: Weak<dyn crate::AnyValue> = Rc::downgrade(inpc);
            let callback = callback.clone();
            Rc::new(move |e: &str| callback((sender.upgrade(), e)))
        };

        let tracked_added = tracked.clone();
        let tracked_removed = tracked.clone();
        let _ = Self::for_each_item(
            collection,
            move |x: &T| {
                let inpc = as_object(x);

                if let Some(model) = ModelTypes::find(&*inpc) {
                    if let Some(notify) = model.as_notify_property_changed(&*inpc) {
                        let token = notify.property_changed().add(handler(&inpc));
                        tracked_added.borrow_mut().push((inpc.clone(), token));
                    }
                }
            },
            move |x: &T| {
                let inpc = as_object(x);

                if let Some(model) = ModelTypes::find(&*inpc) {
                    if let Some(notify) = model.as_notify_property_changed(&*inpc) {
                        let mut tracked = tracked_removed.borrow_mut();
                        if let Some(index) = tracked.iter().position(|(i, _)| **i == *inpc) {
                            let (_, token) = tracked.remove(index);
                            drop(tracked);
                            notify.property_changed().remove(token);
                        }
                    }
                }
            },
            || panic!("Collection reset not supported."),
            false,
        );

        Disposable::create(move || {
            for (i, token) in tracked.borrow().iter() {
                if let Some(notify) = ModelTypes::find(&**i).and_then(|model| model.as_notify_property_changed(&**i)) {
                    notify.property_changed().remove(*token);
                }
            }
        })
    }
}

/// An item as an object: a boxed item is its content, the handle of a shared
/// model object is the object.
fn as_object<T: Clone + PartialEq + 'static>(x: &T) -> BoxedValue {
    if let Some(boxed) = (x as &dyn Any).downcast_ref::<BoxedValue>() {
        return boxed.clone();
    }
    let boxed: BoxedValue = Rc::new(x.clone());
    ValueTypes::reference_object(&boxed).unwrap_or(boxed)
}

/// Subscribes `handler` to a typed collection (through its `add` and
/// `remove` functions) so that the collection holds it weakly: the returned
/// disposable holds it, and disposing or dropping the disposable ends the
/// subscription (a handler that is gone removes its forwarder the next time
/// the collection changes).
pub(crate) fn weak_subscribe<I, C>(
    collection: &C,
    handler: Rc<CollectionChangedHandler<I>>,
    add: fn(&C, Rc<CollectionChangedHandler<I>>) -> u64,
    remove: fn(&C, u64) -> bool,
) -> Rc<dyn IDisposable>
where
    I: 'static,
    C: WeakEventSender,
{
    let weak_handler = Rc::downgrade(&handler);
    let source = collection.downgrade_sender();
    let token = Rc::new(Cell::new(None::<u64>));
    let forwarder_token = token.clone();
    token.set(Some(add(
        collection,
        Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, I>| match weak_handler.upgrade() {
            Some(handler) => handler(e),
            None => {
                if let (Some(collection), Some(token)) = (C::upgrade_sender(&source), forwarder_token.take()) {
                    remove(&collection, token);
                }
            }
        }),
    )));

    let source = collection.downgrade_sender();
    Disposable::create(move || {
        if let (Some(collection), Some(token)) = (C::upgrade_sender(&source), token.take()) {
            remove(&collection, token);
        }
        drop(handler);
    })
}

#[cfg(test)]
mod tests {
    //! Not from upstream: the managed test suite has no tests for these
    //! extensions.

    use super::*;
    use crate::collections::FerroList;
    use crate::data::model::{Event, INotifyPropertyChanged, Model, ModelTypeBuilder};

    fn tracked(list: &FerroList<i32>, weak: bool) -> (Rc<RefCell<Vec<String>>>, Rc<dyn IDisposable>) {
        let log = Rc::new(RefCell::new(Vec::new()));
        let (a, r, z) = (log.clone(), log.clone(), log.clone());
        let subscription = FerroListExtensions::for_each_item_indexed(
            list,
            move |index, item: &i32| a.borrow_mut().push(format!("+{index}:{item}")),
            move |index, item: &i32| r.borrow_mut().push(format!("-{index}:{item}")),
            move || z.borrow_mut().push("reset".to_string()),
            weak,
        );
        (log, subscription)
    }

    #[test]
    fn for_each_item_reports_initial_added_removed_moved_and_reset_items() {
        let list = FerroList::from_items([1, 2, 3]);
        let (log, subscription) = tracked(&list, false);

        list.add(4);
        list.remove_at(0);
        list.move_item(0, 2);
        list.clear();
        list.add(5);
        subscription.dispose();
        list.add(6);

        assert_eq!(
            *log.borrow(),
            ["+0:1", "+1:2", "+2:3", "+3:4", "-0:1", "-0:2", "+2:2", "reset", "+0:5"].map(String::from).to_vec()
        );
    }

    #[test]
    fn weak_for_each_item_ends_when_the_subscription_is_dropped() {
        let list = FerroList::from_items([1]);
        let (log, subscription) = tracked(&list, true);

        list.add(2);
        drop(subscription);
        list.add(3);

        assert_eq!(*log.borrow(), ["+0:1", "+1:2"].map(String::from).to_vec());
        assert!(!list.has_collection_changed_subscribers());
    }

    struct Item {
        property_changed: Event<str>,
    }

    impl PartialEq for Item {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    impl INotifyPropertyChanged for Item {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    impl Model for Item {
        fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
            builder.notify_property_changed()
        }
    }

    #[test]
    fn track_item_property_changed_reports_changes_of_tracked_items() {
        let first = Item::new_model(Item { property_changed: Event::new() });
        let second = Item::new_model(Item { property_changed: Event::new() });
        let list = FerroList::from_items([first.clone()]);
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();

        let subscription = FerroListExtensions::track_item_property_changed(&list, move |(sender, name)| {
            let is_first = sender.is_some_and(|s| s.downcast_ref::<Item>().is_some_and(|s| std::ptr::eq(s, &*first)));
            l.borrow_mut().push((is_first, name.to_string()));
        });
        list.add(second.clone());
        list.get(0).property_changed.raise("Foo");
        second.property_changed.raise("Bar");
        list.remove(&second);
        second.property_changed.raise("Baz");
        subscription.dispose();
        list.get(0).property_changed.raise("Qux");

        assert_eq!(*log.borrow(), vec![(true, "Foo".to_string()), (false, "Bar".to_string())]);
    }
}
