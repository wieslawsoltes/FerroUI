//! A collection that notifies of changes but is not a list cannot be
//! expressed with the items source contract (`IItemsList` is the list), so
//! the reference test for that case has no counterpart.

use crate::items_source::{ItemsChangedEventArgs, ItemsSource};
use crate::ItemsSourceView;
use ferroui_base::collections::FerroList;
use std::rc::Rc;

// Additional Rust integration tests for WinUI's optional stable-key contract.
struct KeyedList {
    keys: std::cell::RefCell<Vec<String>>,
    reads: std::cell::Cell<usize>,
}

impl crate::IKeyIndexMapping for KeyedList {
    fn key_from_index(&self, index: i32) -> String {
        self.keys.borrow()[index as usize].clone()
    }

    fn index_from_key(&self, key: &str) -> i32 {
        self.keys.borrow().iter().position(|candidate| candidate == key).map_or(-1, |i| i as i32)
    }
}

impl crate::IItemsList for KeyedList {
    fn key_index_mapping(&self) -> Option<&dyn crate::IKeyIndexMapping> { Some(self) }
    fn count(&self) -> usize { self.keys.borrow().len() }
    fn get_at(&self, index: usize) -> Option<ferroui_base::BoxedValue> {
        self.reads.set(self.reads.get() + 1);
        Some(Rc::new(self.keys.borrow()[index].clone()))
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

#[test]
fn stable_key_mapping_does_not_materialize_items_and_tracks_reordering() {
    let list = Rc::new(KeyedList {
        keys: std::cell::RefCell::new(vec!["a".into(), "b".into(), "c".into()]),
        reads: std::cell::Cell::new(0),
    });
    let source = ItemsSource::new(list.clone());
    let view = ItemsSourceView::get_or_create(Some(&source));
    assert!(view.has_key_index_mapping());
    assert_eq!(view.key_from_index(1), "b");
    assert_eq!(view.index_from_key("b"), 1);
    assert_eq!(view.index_from_key("absent"), -1);
    list.keys.borrow_mut().swap(0, 1);
    assert_eq!(view.key_from_index(0), "b");
    assert_eq!(view.index_from_key("b"), 0);
    assert_eq!(list.reads.get(), 0);
}

#[test]
fn ordinary_sources_do_not_claim_stable_key_mapping() {
    let source = ItemsSource::from_strs(["a", "b"]);
    assert!(!ItemsSourceView::get_or_create(Some(&source)).has_key_index_mapping());
    assert!(!ItemsSourceView::empty().has_key_index_mapping());
}

#[test]
#[should_panic(expected = "does not implement IKeyIndexMapping")]
fn absent_key_mapping_is_reported_instead_of_inventing_an_index_key() {
    let source = ItemsSource::from_strs(["a"]);
    ItemsSourceView::get_or_create(Some(&source)).key_from_index(0);
}

#[test]
#[should_panic(expected = "Argument index is invalid")]
fn stable_key_mapping_rejects_a_negative_index() {
    let source = ItemsSource::new(Rc::new(KeyedList {
        keys: std::cell::RefCell::new(vec!["a".into()]),
        reads: std::cell::Cell::new(0),
    }));
    ItemsSourceView::get_or_create(Some(&source)).key_from_index(-1);
}

#[test]
fn only_subscribes_to_source_collection_changed_when_collection_changed_subscribed() {
    let source = Rc::new(FerroList::<String>::new());
    let target = ItemsSourceView::get_or_create(Some(&source.clone().into()));

    assert!(!source.has_collection_changed_subscribers());

    let token = target.add_collection_changed(Rc::new(|_: &ItemsChangedEventArgs<'_>| {}));

    assert!(source.has_collection_changed_subscribers());

    target.remove_collection_changed(token);

    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn reassigning_source_unsubscribes_from_previous_source() {
    let source = Rc::new(FerroList::<String>::new());
    let target = ItemsSourceView::new(source.clone().into());

    target.add_collection_changed(Rc::new(|_: &ItemsChangedEventArgs<'_>| {}));

    assert!(source.has_collection_changed_subscribers());

    target.set_source(ItemsSource::from_strs([]));

    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn reassigning_source_subscribes_to_new_source() {
    let source = Rc::new(FerroList::<String>::new());
    let target = ItemsSourceView::new(ItemsSource::from_strs([]));

    target.add_collection_changed(Rc::new(|_: &ItemsChangedEventArgs<'_>| {}));
    target.set_source(source.clone().into());

    assert!(source.has_collection_changed_subscribers());
}

#[test]
fn get_or_create_returns_empty_for_no_source() {
    let target = ItemsSourceView::get_or_create(None);

    assert_eq!(target.count(), 0);
    assert!(Rc::ptr_eq(&target, &ItemsSourceView::empty()));
}

#[test]
fn get_or_create_preserves_an_existing_view_and_its_live_notifications() {
    let source = Rc::new(FerroList::from_items([10, 20]));
    let view = ItemsSourceView::new(source.clone().into());
    let as_source: ItemsSource = view.clone().into();
    let existing = ItemsSourceView::get_or_create(Some(&as_source));
    assert!(Rc::ptr_eq(&view, &existing));
    assert!(as_source.ptr_eq(&view.clone().into()));
    let changes = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = changes.clone();
    let token = as_source.list().add_collection_changed(Rc::new(move |args| {
        observed.borrow_mut().push((args.action, args.new_starting_index));
    })).unwrap();
    source.insert(1, 15);
    assert_eq!(view.count(), 3);
    assert_eq!(existing.index_of(&Some(Rc::new(15))), 1);
    assert_eq!(&*changes.borrow(), &[(ferroui_base::collections::NotifyCollectionChangedAction::Add, 1)]);
    as_source.list().remove_collection_changed(token);
    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn existing_view_as_source_preserves_optional_stable_keys_without_reading_items() {
    let list = Rc::new(KeyedList {
        keys: std::cell::RefCell::new(vec!["a".into(), "b".into()]),
        reads: std::cell::Cell::new(0),
    });
    let view = ItemsSourceView::new(ItemsSource::new(list.clone()));
    let source: ItemsSource = view.clone().into();
    let mapping = source.list().key_index_mapping().unwrap();
    assert_eq!(mapping.key_from_index(1), "b");
    assert_eq!(mapping.index_from_key("missing"), -1);
    assert_eq!(list.reads.get(), 0);
    let ordinary: ItemsSource = ItemsSourceView::new(ItemsSource::from_values([1, 2])).into();
    assert!(ordinary.list().key_index_mapping().is_none());
    assert!(!ordinary.is_notifying());
}

#[test]
fn typed_view_reads_items_as_the_item_type() {
    let source = Rc::new(FerroList::from_items(["foo".to_string(), "bar".to_string()]));
    let target = ItemsSourceView::get_or_create_of::<String>(Some(&source.into()));

    assert_eq!(target.count(), 2);
    assert_eq!(target.get_at(1), Some("bar".to_string()));
    assert_eq!(target.iter().collect::<Vec<_>>(), vec![Some("foo".to_string()), Some("bar".to_string())]);
}

// The tests below are not from the reference: they cover the items views
// carried by the change notifications of the untyped lists.

/// A list that counts the items read from it.
struct CountingList {
    count: usize,
    reads: std::cell::Cell<usize>,
}

impl crate::items_source::IItemsList for CountingList {
    fn count(&self) -> usize {
        self.count
    }

    fn get_at(&self, index: usize) -> Option<ferroui_base::BoxedValue> {
        self.reads.set(self.reads.get() + 1);
        crate::items_source::box_item(&(index as i32))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn window_items_view_reads_items_on_demand() {
    use crate::items_source::{unbox_item, ItemsView};

    let list = Rc::new(CountingList { count: 1000, reads: std::cell::Cell::new(0) });
    let source = ItemsSource::new(list.clone());

    let view = ItemsView::of_source_range(&source, 10, 500);
    assert_eq!(view.len(), 500);
    assert!(!view.is_empty());
    assert_eq!(view.iter().len(), 500);
    assert_eq!(list.reads.get(), 0);

    assert_eq!(unbox_item::<i32>(&view.get(2)), Some(12));
    assert_eq!(list.reads.get(), 1);

    let first: Vec<_> = view.iter().take(3).map(|i| unbox_item::<i32>(&i).unwrap()).collect();
    assert_eq!(first, [10, 11, 12]);
    assert_eq!(view.iter().next_back().and_then(|i| unbox_item::<i32>(&i)), Some(509));
    assert_eq!(list.reads.get(), 5);

    assert_eq!(ItemsView::of_source(&source).to_vec().len(), 1000);
    assert_eq!(list.reads.get(), 1005);
}

#[test]
fn replacing_the_source_of_an_item_collection_reads_no_items() {
    use crate::items_source::unbox_item;
    use ferroui_base::collections::NotifyCollectionChangedAction;
    use std::cell::RefCell;

    let _scope = crate::test_support::test_scope();
    let target = crate::ItemsControl::new();
    let old = Rc::new(CountingList { count: 3, reads: std::cell::Cell::new(0) });
    let new = Rc::new(CountingList { count: 5, reads: std::cell::Cell::new(0) });
    target.set_items_source(Some(ItemsSource::new(old.clone())));

    let raised = Rc::new(RefCell::new(Vec::new()));
    let r = raised.clone();
    target.items_view().add_collection_changed(Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
        r.borrow_mut().push((e.action, e.old_items.len(), e.new_items.len(), e.old_starting_index, e.new_starting_index));
    }));

    target.set_items_source(Some(ItemsSource::new(new.clone())));

    assert_eq!(
        *raised.borrow(),
        [(NotifyCollectionChangedAction::Remove, 3, 0, 0, -1), (NotifyCollectionChangedAction::Add, 0, 5, -1, 0)]
    );
    assert_eq!(old.reads.get(), 0);
    assert_eq!(new.reads.get(), 0);

    // The removed items are those of the old source, read from it on demand
    // after the source has been replaced.
    let removed = Rc::new(RefCell::new(Vec::new()));
    let r = removed.clone();
    target.items_view().add_collection_changed(Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
        r.borrow_mut().extend(e.old_items.iter().map(|i| unbox_item::<i32>(&i).unwrap()));
    }));
    target.set_items_source(Some(ItemsSource::new(old.clone())));
    assert_eq!(*removed.borrow(), [0, 1, 2, 3, 4]);
    assert_eq!(new.reads.get(), 5);
    assert_eq!(old.reads.get(), 0);
}

#[test]
fn typed_list_changes_box_items_on_demand() {
    use crate::items_source::{unbox_item, IItemsList};
    use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
    use std::cell::RefCell;

    let list = Rc::new(FerroList::from_items(["a".to_string(), "b".to_string()]));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let s = seen.clone();
    let token = IItemsList::add_collection_changed(
        &*list,
        Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
            let new: Vec<String> = e.new_items.iter().map(|i| unbox_item::<String>(&i).unwrap()).collect();
            let old: Vec<String> = e.old_items.to_vec().iter().map(|i| unbox_item::<String>(i).unwrap()).collect();
            s.borrow_mut().push((e.action, old, new, e.old_starting_index, e.new_starting_index));
        }),
    );
    assert!(token.is_some());

    list.add_range(["c".to_string(), "d".to_string()]);
    list.remove_at(0);

    assert_eq!(
        *seen.borrow(),
        [
            (NotifyCollectionChangedAction::Add, vec![], vec!["c".to_string(), "d".to_string()], -1, 2),
            (NotifyCollectionChangedAction::Remove, vec!["a".to_string()], vec![], 0, -1),
        ]
    );
}
