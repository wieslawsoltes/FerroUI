//! Port of the upstream tests of the list that can be mutated while it is enumerated.
//!
//! The port has no class of its own for it: what the class adds to the list is the behaviour of every
//! [`FerroList`] (`collections/ferro_list.rs`). An enumeration is a snapshot of the storage (`snapshot`), which is
//! the enumerator of upstream: while one is alive a mutation copies the storage first and is applied to the copy,
//! and a mutation made while none is alive is applied in place. The tests are upstream's on `FerroList`: a
//! `foreach` is a loop over a snapshot, and the identity of the inner list is the identity of the storage
//! (`inner_for_tests`).

use crate::collections::{
    FerroList, IFerroReadOnlyList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs,
};
use crate::data::model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn strings<const N: usize>(items: [&str; N]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

fn list<const N: usize>(items: [&str; N]) -> FerroList<String> {
    let target = FerroList::new();
    for item in items {
        target.add(item.to_string());
    }
    target
}

#[test]
fn list_is_not_copied_outside_enumeration() {
    let target = FerroList::<String>::new();
    let inner = target.inner_for_tests();

    target.add("foo".to_string());
    target.add("bar".to_string());
    target.remove(&"foo".to_string());
    target.insert(0, "baz".to_string());
    target.set(0, "qux".to_string());
    target.move_item(0, 1);
    target.clear();

    assert_eq!(inner, target.inner_for_tests());
}

#[test]
fn list_is_copied_when_mutated_during_enumeration() {
    let target = FerroList::<String>::new();
    let inner = target.inner_for_tests();

    target.add("foo".to_string());

    for item in target.snapshot().iter() {
        assert_eq!(inner, target.inner_for_tests());
        target.add("bar".to_string());
        assert_ne!(inner, target.inner_for_tests());
        assert_eq!("foo", item);
    }

    assert_eq!(strings(["foo", "bar"]), target.to_vec());
}

#[test]
fn enumerator_iterates_snapshot_taken_at_creation() {
    let target = list(["foo", "bar", "baz"]);
    let mut seen = Vec::new();

    for item in target.snapshot().iter() {
        seen.push(item.clone());
        target.remove(item);
    }

    assert_eq!(strings(["foo", "bar", "baz"]), seen);
    assert!(target.is_empty());
}

#[test]
fn list_is_not_copied_after_enumeration() {
    let target = FerroList::<String>::new();
    let mut inner = target.inner_for_tests();

    target.add("foo".to_string());

    for item in target.snapshot().iter() {
        target.add("bar".to_string());
        assert_ne!(inner, target.inner_for_tests());
        inner = target.inner_for_tests();
        assert_eq!("foo", item);
    }

    target.add("baz".to_string());
    assert_eq!(inner, target.inner_for_tests());
}

#[test]
fn list_is_copied_only_once_during_enumeration() {
    let target = FerroList::<String>::new();
    let mut inner = target.inner_for_tests();

    target.add("foo".to_string());

    for _item in target.snapshot().iter() {
        target.add("bar".to_string());
        assert_ne!(inner, target.inner_for_tests());
        inner = target.inner_for_tests();
        target.add("baz".to_string());
        assert_eq!(inner, target.inner_for_tests());
    }
}

#[test]
fn list_is_copied_during_nested_enumerations() {
    let target = FerroList::<String>::new();
    let initial_inner = target.inner_for_tests();
    let mut first_items = Vec::new();
    let mut second_items = Vec::new();

    target.add("foo".to_string());

    for i in target.snapshot().iter() {
        target.add("bar".to_string());

        let first_inner = target.inner_for_tests();
        assert_ne!(initial_inner, first_inner);

        for j in target.snapshot().iter() {
            target.add("baz".to_string());

            let second_inner = target.inner_for_tests();
            assert_ne!(first_inner, second_inner);

            second_items.push(j.clone());
        }

        first_items.push(i.clone());
    }

    assert_eq!(strings(["foo"]), first_items);
    assert_eq!(strings(["foo", "bar"]), second_items);
    assert_eq!(strings(["foo", "bar", "baz", "baz"]), target.to_vec());

    let final_inner = target.inner_for_tests();
    target.add("final".to_string());
    assert_eq!(final_inner, target.inner_for_tests());
}

/// Upstream enumerates the list as an `IEnumerable<string>`; the interface of the port that enumerates is the
/// read-only list.
#[test]
fn enumeration_through_interface_is_safe() {
    let target = list(["foo", "bar"]);
    let mut seen = Vec::new();

    let interface: &dyn IFerroReadOnlyList<String> = &target;
    for item in interface.snapshot().iter() {
        seen.push(item.clone());
        target.add("baz".to_string());
    }

    assert_eq!(strings(["foo", "bar"]), seen);
    assert_eq!(strings(["foo", "bar", "baz", "baz"]), target.to_vec());
}

#[test]
fn collection_changed_is_raised_for_mutations_after_copy() {
    let target = list(["foo"]);
    let events = Rc::new(RefCell::new(Vec::new()));

    let log = events.clone();
    target.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, String>| {
        log.borrow_mut().push((e.action, e.new_items.to_vec()));
    }));

    for _item in target.snapshot().iter() {
        target.add("bar".to_string());
    }

    target.add("baz".to_string());

    let events = events.borrow();
    assert_eq!(2, events.len());
    assert!(events.iter().all(|e| e.0 == NotifyCollectionChangedAction::Add));
    assert_eq!(strings(["bar"]), events[0].1);
    assert_eq!(strings(["baz"]), events[1].1);
}

#[test]
fn property_changed_is_raised_for_count_after_copy() {
    let target = list(["foo"]);
    let count_changed = Rc::new(Cell::new(0));

    let counter = count_changed.clone();
    target.property_changed().add(Rc::new(move |e: &str| {
        if e == "Count" {
            counter.set(counter.get() + 1);
        }
    }));

    for _i in target.snapshot().iter() {
        target.add("bar".to_string());
    }

    target.add("baz".to_string());

    assert_eq!(2, count_changed.get());
}

#[test]
fn validator_is_invoked_after_copy() {
    let target = list(["foo"]);
    let validated = Rc::new(RefCell::new(Vec::new()));

    let log = validated.clone();
    target.set_validate(Some(Rc::new(move |item: &String| log.borrow_mut().push(item.clone()))));

    for _item in target.snapshot().iter() {
        target.add("bar".to_string());
    }

    target.add("baz".to_string());

    assert_eq!(strings(["bar", "baz"]), *validated.borrow());
}
