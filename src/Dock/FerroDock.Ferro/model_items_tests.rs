// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original list mutation/identity/change contents at the native ItemsSource boundary.
use super::*;
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_system::{
    collections::{
        object_model::ObservableCollection,
        specialized::{CollectionChange, INotifyCollectionChanged},
    },
    events::Event,
};
use std::cell::{Cell, RefCell};
fn doc(id: &str) -> Rc<dyn IDockable> {
    let d = Document::new();
    d.set_id(id.into());
    d
}
#[test]
fn observable_projection_forwards_original_action_indices_and_exact_items_without_mirror_state() {
    crate::register_types();
    let first = doc("first");
    let second = doc("second");
    let third = doc("third");
    let source = ObservableCollection::from_items(vec![first.clone()]);
    let list = ListHandle::from_observable(source.clone());
    let native = items(&list).unwrap();
    let other = items(&ListHandle::from_observable(source.clone())).unwrap();
    assert_eq!(native, other);
    assert_eq!(native.list().identity(), Some(list.identity()));
    assert!(std::ptr::eq(
        list.as_notify_collection_changed()
            .unwrap()
            .collection_changed(),
        source.collection_changed()
    ));
    assert!(std::ptr::eq(
        list.as_notify_property_changed()
            .unwrap()
            .property_changed(),
        ferroui_system::component_model::INotifyPropertyChanged::property_changed(&source)
    ));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let token = native
        .list()
        .add_collection_changed(Rc::new(move |e| {
            let ids = |items: ItemsView<'_>| {
                (0..items.len())
                    .map(|i| {
                        items
                            .get(i)
                            .unwrap()
                            .downcast_ref::<Rc<dyn IDockable>>()
                            .unwrap()
                            .id()
                    })
                    .collect::<Vec<_>>()
            };
            seen.borrow_mut().push((
                e.action,
                ids(e.new_items),
                ids(e.old_items),
                e.new_starting_index,
                e.old_starting_index,
            ));
        }))
        .unwrap();
    list.add(second.clone());
    list.set(0, third.clone());
    source.move_item(0, 1);
    list.remove_at(0);
    list.clear();
    use ferroui_system::collections::specialized::NotifyCollectionChangedAction::*;
    assert_eq!(
        &*changes.borrow(),
        &[
            (Add, vec!["second".into()], vec![], 1, -1),
            (Replace, vec!["third".into()], vec!["first".into()], 0, 0),
            (Move, vec!["third".into()], vec!["third".into()], 1, 0),
            (Remove, vec![], vec!["second".into()], -1, 0),
            (Reset, vec![], vec![], -1, -1)
        ]
    );
    native.list().remove_collection_changed(token);
    list.add(first.clone());
    assert_eq!(changes.borrow().len(), 5);
    let boxed = native.list().get_at(0).unwrap();
    assert!(Rc::ptr_eq(
        boxed.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
        &first
    ));
    assert_eq!(native.list().index_of(&Some(boxed)), 0);
    assert_eq!(
        native.list().index_of(&Some(Rc::new("wrong".to_owned()))),
        -1
    );
}
#[test]
fn original_mutable_ilist_view_reorders_same_plain_list_and_keeps_custom_comparer_and_enumerator() {
    crate::register_types();
    let first = doc("first");
    let second = doc("second");
    let list = ListHandle::from_plain_with_comparer(
        vec![Some(first.clone()), Some(second.clone())],
        Rc::new(|a, b| {
            a.as_ref().expect("nonnull comparer fixture").id()
                == b.as_ref().expect("nonnull comparer fixture").id()
        }),
    );
    let source = items(&list).unwrap();
    assert!(!source.list().is_notifying());
    let equivalent = doc("first");
    assert_eq!(source.list().index_of(&Some(Rc::new(equivalent))), 0);
    let converted = ValueTypes::try_convert_for_property(
        source.list().as_any(),
        std::any::TypeId::of::<ListHandle<Option<BoxedValue>>>(),
    )
    .unwrap();
    let view = converted
        .downcast_ref::<ListHandle<Option<BoxedValue>>>()
        .unwrap();
    assert_eq!(view.identity(), list.identity());
    let first_value = view.get(0);
    view.remove_at(0);
    view.insert(1, first_value);
    assert!(Rc::ptr_eq(
        list.get(0).as_ref().expect("nonnull identity fixture"),
        &second
    ));
    assert!(Rc::ptr_eq(
        list.get(1).as_ref().expect("nonnull identity fixture"),
        &first
    ));
    let alias = mutable(&list);
    assert!(view.ptr_eq(&alias));
    let mut iterator = view.iter();
    assert!(iterator.next().is_some());
    list.add(Some(doc("third")));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| iterator.next())).is_err());
    view.clear();
    assert!(list.is_empty());
    let native: BoxedValue = Rc::new(list.clone());
    assert!(
        ValueTypes::try_convert(Some(&native), ValueType::of::<Option<ItemsSource>>()).is_some()
    );
}
struct TypedList {
    items: ObservableCollection<Option<Rc<dyn IDockable>>>,
    subscriptions: Cell<usize>,
}
impl IList<Option<Rc<dyn IDockable>>> for TypedList {
    fn count(&self) -> usize {
        self.items.count()
    }
    fn get(&self, i: usize) -> Option<Rc<dyn IDockable>> {
        self.items.get(i)
    }
    fn set(&self, i: usize, v: Option<Rc<dyn IDockable>>) {
        self.items.set(i, v);
    }
    fn insert(&self, i: usize, v: Option<Rc<dyn IDockable>>) {
        self.items.insert(i, v);
    }
    fn remove_at(&self, i: usize) {
        self.items.remove_at(i);
    }
    fn clear(&self) {
        self.items.clear();
    }
    fn index_of(&self, v: &Option<Rc<dyn IDockable>>) -> Option<usize> {
        self.items.index_of(v)
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(&self.items)
    }
    fn has_typed_collection_changes(&self) -> bool {
        true
    }
    fn add_collection_changed(
        &self,
        h: Rc<ferrodock_model::collections::ReadOnlyListChanged<Option<Rc<dyn IDockable>>>>,
    ) -> Option<u64> {
        self.subscriptions.set(self.subscriptions.get() + 1);
        Some(self.items.add_collection_changed(h))
    }
    fn remove_collection_changed(&self, t: u64) {
        if self.items.remove_collection_changed(t) {
            self.subscriptions.set(self.subscriptions.get() - 1);
        }
    }
}
#[test]
fn custom_typed_source_retains_notifications_and_explicit_unsubscribe_for_both_views() {
    crate::register_types();
    let backend = Rc::new(TypedList {
        items: ObservableCollection::new(),
        subscriptions: Cell::new(0),
    });
    let list = ListHandle::from_list(backend.clone());
    let native = items(&list).unwrap();
    let view = mutable(&list);
    assert!(view.has_typed_collection_changes());
    let observed = Rc::new(RefCell::new(Vec::new()));
    let seen = observed.clone();
    let token = view
        .add_collection_changed(Rc::new(move |e| seen.borrow_mut().push(e.new_items.len())))
        .unwrap();
    let seen = observed.clone();
    let ui = native
        .list()
        .add_collection_changed(Rc::new(move |e| seen.borrow_mut().push(e.new_items.len())))
        .unwrap();
    assert_eq!(backend.subscriptions.get(), 2);
    list.add(Some(doc("added")));
    assert_eq!(&*observed.borrow(), &[1, 1]);
    view.remove_collection_changed(token);
    native.list().remove_collection_changed(ui);
    assert_eq!(backend.subscriptions.get(), 0);
}
struct CountsOnly {
    items: ListHandle<Option<Rc<dyn IDockable>>>,
    changed: Event<CollectionChange>,
}
impl INotifyCollectionChanged for CountsOnly {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.changed
    }
}
impl IList<Option<Rc<dyn IDockable>>> for CountsOnly {
    fn count(&self) -> usize {
        self.items.count()
    }
    fn get(&self, i: usize) -> Option<Rc<dyn IDockable>> {
        self.items.get(i)
    }
    fn set(&self, i: usize, v: Option<Rc<dyn IDockable>>) {
        self.items.set(i, v);
    }
    fn insert(&self, i: usize, v: Option<Rc<dyn IDockable>>) {
        self.items.insert(i, v);
    }
    fn remove_at(&self, i: usize) {
        self.items.remove_at(i);
    }
    fn clear(&self) {
        self.items.clear();
    }
    fn index_of(&self, v: &Option<Rc<dyn IDockable>>) -> Option<usize> {
        self.items.index_of(v)
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(self)
    }
}
#[test]
fn counts_only_custom_notifications_are_explicitly_rejected_instead_of_inventing_change_items() {
    let source = ListHandle::from_list(Rc::new(CountsOnly {
        items: ListHandle::from_plain(vec![Some(doc("a"))]),
        changed: Event::new(),
    }));
    assert!(!source.has_typed_collection_changes());
    assert!(items(&source).is_none());
    assert!(source.add_collection_changed(Rc::new(|_| {})).is_none());
    source.remove_collection_changed(1);
    assert!(source.as_notify_property_changed().is_none());
}

#[test]
fn nullable_projection_keeps_slot_indices_null_event_payloads_and_original_list_identity() {
    crate::register_types();
    let first = doc("first");
    let second = doc("second");
    let source = ObservableCollection::from_items(vec![None, Some(first.clone())]);
    let list = ListHandle::from_observable(source.clone());
    let native = items(&list).unwrap();
    let mutable = mutable(&list);
    assert_eq!(native.list().identity(), Some(list.identity()));
    assert_eq!(mutable.identity(), list.identity());
    assert_eq!(native.list().count(), 2);
    assert!(native.list().get_at(0).is_none());
    assert_eq!(native.list().index_of(&None), 0);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let token = native
        .list()
        .add_collection_changed(Rc::new(move |event| {
            let ids = |items: ItemsView<'_>| {
                (0..items.len())
                    .map(|index| {
                        items
                            .get(index)
                            .map(|value| value.downcast_ref::<Rc<dyn IDockable>>().unwrap().id())
                    })
                    .collect::<Vec<_>>()
            };
            seen.borrow_mut().push((
                event.action,
                ids(event.new_items),
                ids(event.old_items),
                event.new_starting_index,
                event.old_starting_index,
            ));
        }))
        .unwrap();
    list.add(None);
    list.set(0, Some(second.clone()));
    source.move_item(2, 0);
    list.remove_at(0);
    mutable.insert(1, None);
    mutable.set(0, None);
    use ferroui_system::collections::specialized::NotifyCollectionChangedAction::*;
    assert_eq!(
        &*changes.borrow(),
        &[
            (Add, vec![None], vec![], 2, -1),
            (Replace, vec![Some("second".into())], vec![None], 0, 0),
            (Move, vec![None], vec![None], 0, 2),
            (Remove, vec![], vec![None], -1, 0),
            (Add, vec![None], vec![], 1, -1),
            (Replace, vec![None], vec![Some("second".into())], 0, 0),
        ]
    );
    assert_eq!(list.count(), 3);
    let mut cursor = mutable.iter();
    assert!(matches!(cursor.next(), Some(None)));
    assert!(matches!(cursor.next(), Some(None)));
    let value = cursor.next().unwrap().unwrap();
    assert!(Rc::ptr_eq(
        value.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
        &first
    ));
    assert!(cursor.next().is_none());
    native.list().remove_collection_changed(token);
}
