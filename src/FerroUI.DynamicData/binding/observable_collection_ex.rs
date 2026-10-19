//! `ObservableCollectionEx` (`Binding/ObservableCollectionEx.cs`): the changes of an observable
//! collection as an observable, and as change sets.

use super::{ObservableCollectionExtended, ReadOnlyObservableCollection};
use crate::list::{ChangeAwareList, ChangeSet, IExtendedList, ObservableListEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::EventPattern;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::collections::specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, SharedNotifyCollectionChangedEventArgs};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// A notification of a change of a collection (`EventPattern<NotifyCollectionChangedEventArgs>`):
/// the collection that changed, and the description of the change.
pub type CollectionChangedEventPattern<T> = EventPattern<ObservableCollection<T>, SharedNotifyCollectionChangedEventArgs<T>>;

/// The changes of an observable collection as an observable (`ObservableCollectionEx`), for
/// `ObservableCollection<T>`, `ObservableCollectionExtended<T>` and
/// `ReadOnlyObservableCollection<T>`.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's generic form takes any
/// `INotifyCollectionChanged` that enumerates its items; here the collections are those that notify
/// their items, which the untyped `INotifyCollectionChanged` of `ferroui-system` does not.
pub trait ObservableCollectionEx<T: Clone + PartialEq + 'static> {
    /// The observable collection whose changes are observed.
    fn notifying_collection(&self) -> ObservableCollection<T>;

    /// The notifications of the changes of the collection (`ObserveCollectionChanges`).
    fn observe_collection_changes(&self) -> Rc<dyn IObservable<CollectionChangedEventPattern<T>>> {
        observe_collection_changes(self.notifying_collection())
    }

    /// The items of the collection as change sets of a list: its items first, then each of its
    /// changes (`ToObservableChangeSet<T>()`, `ToObservableChangeSet<TCollection, T>()`).
    fn to_observable_change_set(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        to_observable_change_set(self.notifying_collection())
    }

    /// [`to_observable_change_set`](Self::to_observable_change_set) with each item keyed by
    /// `key_selector` (`ToObservableChangeSet<TObject, TKey>(Func<TObject, TKey>)`).
    fn to_observable_change_set_with_key<K: Eq + Hash + Clone + 'static>(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        self.to_observable_change_set().add_key(key_selector)
    }
}

impl<T: Clone + PartialEq + 'static> ObservableCollectionEx<T> for ObservableCollection<T> {
    fn notifying_collection(&self) -> ObservableCollection<T> {
        self.clone()
    }
}

impl<T: Clone + PartialEq + 'static> ObservableCollectionEx<T> for ObservableCollectionExtended<T> {
    fn notifying_collection(&self) -> ObservableCollection<T> {
        self.collection().clone()
    }
}

impl<T: Clone + PartialEq + 'static> ObservableCollectionEx<T> for ReadOnlyObservableCollection<T> {
    fn notifying_collection(&self) -> ObservableCollection<T> {
        self.collection().clone()
    }
}

fn observe_collection_changes<T: Clone + PartialEq + 'static>(source: ObservableCollection<T>) -> Rc<dyn IObservable<CollectionChangedEventPattern<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<CollectionChangedEventPattern<T>>>| {
        let sender = source.clone();
        let token = source.add_collection_changed(Rc::new(move |args: &NotifyCollectionChangedEventArgs<'_, T>| {
            observer.on_next(EventPattern::new(sender.clone(), args.share()));
        }));
        let source = source.clone();
        Disposable::create(move || {
            source.remove_collection_changed(token);
        })
    })
}

fn to_observable_change_set<T: Clone + PartialEq + 'static>(source: ObservableCollection<T>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let data = Rc::new(RefCell::new(ChangeAwareList::from_items(source.to_vec())));

        let initial = data.borrow_mut().capture_changes();
        observer.on_next(initial);

        let collection = source.clone();
        observe_collection_changes(source.clone()).subscribe_next(move |args: CollectionChangedEventPattern<T>| {
            let changes = args.event_args;
            let captured = {
                let mut list = data.borrow_mut();
                apply(&mut list, &changes, &collection);
                list.capture_changes()
            };
            observer.on_next(captured);
        })
    })
}

/// Applies a change of the collection to the list that mirrors it (the accumulator of
/// `ToObservableChangeSet`).
fn apply<T: Clone + PartialEq + 'static>(list: &mut ChangeAwareList<T>, changes: &SharedNotifyCollectionChangedEventArgs<T>, source: &ObservableCollection<T>) {
    match changes.action() {
        NotifyCollectionChangedAction::Add => {
            let new_index = if changes.new_starting_index() == -1 { list.count() } else { changes.new_starting_index() as usize };
            if changes.new_items().len() == 1 {
                list.insert(new_index, changes.new_items()[0].clone());
            } else {
                IExtendedList::insert_range(list, changes.new_items().to_vec(), new_index);
            }
        }
        NotifyCollectionChangedAction::Remove => {
            if changes.old_starting_index() == -1 {
                for item in changes.old_items() {
                    list.remove(item);
                }
            } else if changes.old_items().len() == 1 {
                list.remove_at(changes.old_starting_index() as usize);
            } else {
                list.remove_range(changes.old_starting_index() as usize, changes.old_items().len());
            }
        }
        NotifyCollectionChangedAction::Replace => {
            if let Some(replaced_item) = changes.new_items().first() {
                if changes.new_starting_index() == -1 {
                    let original = changes.old_items().first().expect("a replacement has the item it replaced");
                    let old_index = list.index_of(original).expect("the replaced item is in the list");
                    list.set(old_index, replaced_item.clone());
                } else {
                    list.set(changes.new_starting_index() as usize, replaced_item.clone());
                }
            }
        }
        NotifyCollectionChangedAction::Reset => {
            list.clear();
            list.add_range(source.to_vec());
        }
        NotifyCollectionChangedAction::Move => {
            // `UnspecifiedIndexException`.
            assert!(changes.old_starting_index() != -1, "Move -> OldStartingIndex");
            assert!(changes.new_starting_index() != -1, "Move -> NewStartingIndex");
            IExtendedList::move_index(list, changes.old_starting_index() as usize, changes.new_starting_index() as usize);
        }
    }
}

