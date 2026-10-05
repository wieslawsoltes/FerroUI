//! A collection that notifies of changes but is not a list cannot be
//! expressed with the items source contract (`IItemsList` is the list), so
//! the reference test for that case has no counterpart.

use crate::items_source::{ItemsChangedEventArgs, ItemsSource};
use crate::ItemsSourceView;
use ferroui_base::collections::FerroList;
use std::rc::Rc;

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
