use super::*;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::collections::specialized::{
    INotifyCollectionChanged, NotifyCollectionChangedAction as Action,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[test]
fn read_only_view_is_live_keeps_identity_and_rejects_every_mutator() {
    let source = ObservableCollection::from_items(vec![1, 2]);
    let view = ReadOnlyObservableCollection::new(source.clone());
    let alias = view.clone();
    let other = ReadOnlyObservableCollection::new(source.clone());
    assert!(alias.ptr_eq(&view));
    assert!(!other.ptr_eq(&view));
    source.add(3);
    assert_eq!(view.count(), 3);
    assert!(!view.is_empty());
    assert_eq!(view.get(2), 3);
    assert!(view.contains(&2));
    assert_eq!(view.index_of(&2), Some(1));
    assert_eq!(view.index_of(&9), None);
    assert_eq!(view.iter().collect::<Vec<_>>(), [1, 2, 3]);
    let mut buffer = [0; 5];
    view.copy_to(&mut buffer, 1);
    assert_eq!(buffer, [0, 1, 2, 3, 0]);
    let snapshot = view.to_vec();
    source.add(4);
    assert_eq!(snapshot, [1, 2, 3]);
    let list = view.as_list_handle();
    assert!(list.is_read_only());
    assert!(list.ptr_eq(&view.as_list_handle()));
    assert!(list.as_observable_collection().is_none());
    let changes: Vec<Box<dyn Fn()>> = vec![
        Box::new(|| list.add(5)),
        Box::new(|| list.insert(999, 5)),
        Box::new(|| list.set(0, 5)),
        Box::new(|| list.remove_at(0)),
        Box::new(|| list.clear()),
        Box::new(|| {
            list.remove(&999);
        }),
    ];
    for change in changes {
        assert!(catch_unwind(AssertUnwindSafe(change)).is_err());
    }
    assert_eq!(source.to_vec(), [1, 2, 3, 4]);
    source.clear();
    assert!(view.is_empty());
}

#[test]
fn read_only_forwards_all_change_payloads_property_order_and_cross_form_registration_order() {
    let source = ObservableCollection::from_items(vec![1, 2]);
    let view = ReadOnlyObservableCollection::new(source.clone());
    let order = Rc::new(RefCell::new(Vec::new()));
    let seen = order.clone();
    view.property_changed()
        .add(Rc::new(move |name| seen.borrow_mut().push(name.to_owned())));
    let seen = order.clone();
    view.collection_changed()
        .add(Rc::new(move |_| seen.borrow_mut().push("untyped".into())));
    let payloads = Rc::new(RefCell::new(Vec::new()));
    let log = payloads.clone();
    let seen = order.clone();
    let token = view.add_collection_changed(Rc::new(move |args| {
        seen.borrow_mut().push("typed".into());
        log.borrow_mut().push((
            args.action,
            args.new_items.to_vec(),
            args.old_items.to_vec(),
            args.new_starting_index,
            args.old_starting_index,
        ));
    }));
    source.add(3);
    assert_eq!(&*order.borrow(), &["Count", "Item[]", "untyped", "typed"]);
    order.borrow_mut().clear();
    source.set(0, 9);
    source.move_item(2, 0);
    source.remove_at(1);
    source.clear();
    assert_eq!(
        &*payloads.borrow(),
        &[
            (Action::Add, vec![3], vec![], 2, -1),
            (Action::Replace, vec![9], vec![1], 0, 0),
            (Action::Move, vec![3], vec![3], 0, 2),
            (Action::Remove, vec![], vec![9], -1, 1),
            (Action::Reset, vec![], vec![], -1, -1)
        ]
    );
    assert!(view.remove_collection_changed(token));
    source.add(7);
    assert_eq!(payloads.borrow().len(), 5);
}

#[test]
fn read_only_has_exactly_one_source_collection_subscription_despite_many_view_listeners() {
    let source = ObservableCollection::new();
    let view = ReadOnlyObservableCollection::new(source.clone());
    let nested = source.clone();
    let once = Cell::new(false);
    view.add_collection_changed(Rc::new(move |_| {
        if !once.replace(true) {
            nested.add(2);
        }
    }));
    view.add_collection_changed(Rc::new(|_| {}));
    view.collection_changed().add(Rc::new(|_| {}));
    source.add(1);
    assert_eq!(source.to_vec(), [1, 2]);
    let extra = ReadOnlyObservableCollection::new(source.clone());
    let nested = source.clone();
    let token = view.add_collection_changed(Rc::new(move |_| {
        nested.add(4);
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| source.add(3))).is_err());
    view.remove_collection_changed(token);
    drop(extra);
}

#[test]
fn dropping_read_only_unsubscribes_without_retaining_a_wrapper_cycle() {
    let source = ObservableCollection::new();
    let view = ReadOnlyObservableCollection::new(source.clone());
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    view.property_changed().add(Rc::new(move |_| {
        let _ = &capture;
    }));
    drop(view);
    assert!(weak.upgrade().is_none());
    // If the wrapper's handler remained, this second source handler would make
    // nested mutation illegal. The native wrapper has detached both forwards.
    let nested = source.clone();
    let once = Cell::new(false);
    source.add_collection_changed(Rc::new(move |_| {
        if !once.replace(true) {
            nested.add(2);
        }
    }));
    source.add(1);
    assert_eq!(source.to_vec(), [1, 2]);
}
