//! `GroupFixture` (`DynamicData.Tests/Cache/GroupFixture.cs`).

use crate::domain::Person;
use ferroui_dynamic_data::binding::ObservableCollectionExtended;
use ferroui_dynamic_data::cache::{ChangeReason, Group, GroupChangeSet, IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The source of the tests (`_source`), keyed by name.
fn new_source() -> SourceCache<Rc<Person>, String> {
    SourceCache::new(|p: &Rc<Person>| p.name())
}

/// `Mytype`: a key and a grouping.
struct MyType {
    key: i32,
    grouping: String,
}

impl MyType {
    fn new(key: i32, grouping: &str) -> Rc<Self> {
        Rc::new(Self { key, grouping: grouping.to_owned() })
    }
}

#[test]
fn kaboom() {
    let cache = SourceCache::new(|x: &Rc<MyType>| x.key);
    let list_with_duplicates = vec![MyType::new(1, "G1"), MyType::new(1, "G2")];
    let _subscription = cache.connect().group(|x: &Rc<MyType>| x.grouping.clone()).subscribe_ignore();

    cache.edit(|x| {
        x.add_or_update_items(&mut list_with_duplicates.clone().into_iter());
        x.clear();
        x.add_or_update_items(&mut list_with_duplicates.clone().into_iter());
    });
}

#[test]
fn add() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).subscribe_next(move |updates: GroupChangeSet<Rc<Person>, String, i32>| {
        assert_eq!(updates.count(), 1, "Should be 1 add");
        assert_eq!(updates.changes()[0].reason(), ChangeReason::Add);
        target.set(true);
    });
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 20)));

    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

/// An entry of a group (`GroupEntryViewModel`).
struct GroupEntryViewModel {
    #[allow(dead_code)]
    person: Rc<Person>,
}

impl PartialEq for GroupEntryViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A group with its entries bound to a collection (`GroupViewModel`).
struct GroupViewModel {
    entries: ObservableCollectionExtended<Rc<GroupEntryViewModel>>,
}

impl GroupViewModel {
    fn new(group: &Group<Rc<Person>, String, String>) -> Rc<Self> {
        let entries = ObservableCollectionExtended::new();
        let _subscription = group.cache().connect().transform(|x: &Rc<Person>| Rc::new(GroupEntryViewModel { person: x.clone() })).bind(&entries).subscribe_ignore();
        Rc::new(Self { entries })
    }

    /// `Entries`.
    fn entries(&self) -> &ObservableCollectionExtended<Rc<GroupEntryViewModel>> {
        &self.entries
    }
}

impl PartialEq for GroupViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn first_letter(person: &Rc<Person>) -> String {
    person.name().chars().next().map(String::from).unwrap_or_default()
}

#[test]
fn add_item_after_update_item_process_add() {
    let source = new_source();
    let entries = ObservableCollectionExtended::<Rc<GroupViewModel>>::new();
    let subscriber = source.connect().group(first_letter).transform(|x: &Group<Rc<Person>, String, String>| GroupViewModel::new(x)).bind(&entries).subscribe_ignore();

    source.edit(|x| x.add_or_update(Person::new("Adam", 1)));

    let first_group = entries.get(0);
    assert_eq!(first_group.entries().count(), 1);

    source.edit(|x| {
        x.add_or_update(Person::new("Adam", 3)); // update
        x.add_or_update(Person::new("Alfred", 1)); // add
    });

    assert_eq!(first_group.entries().count(), 2);

    subscriber.dispose();
    source.dispose();
}

#[test]
fn fires_completed_when_disposed() {
    let source = new_source();
    let completed = Rc::new(Cell::new(false));
    let target = completed.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).subscribe_next_completed(|_| {}, move || target.set(true));
    source.dispose();
    subscriber.dispose();
    assert!(completed.get());
}

#[test]
fn fires_many_value_for_batch_of_different_adds() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).subscribe_next(move |updates: GroupChangeSet<Rc<Person>, String, i32>| {
        assert_eq!(updates.count(), 4, "Should be 4 adds");
        for update in &updates {
            assert_eq!(update.reason(), ChangeReason::Add);
        }
        target.set(true);
    });
    source.edit(|updater| {
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person2", 21));
        updater.add_or_update(Person::new("Person3", 22));
        updater.add_or_update(Person::new("Person4", 23));
    });

    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

#[test]
fn fires_only_once_for_a_batch_of_unique_values() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).subscribe_next(move |updates: GroupChangeSet<Rc<Person>, String, i32>| {
        assert_eq!(updates.count(), 1, "Should be 1 add");
        assert_eq!(updates.changes()[0].reason(), ChangeReason::Add);
        target.set(true);
    });
    source.edit(|updater| {
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person2", 20));
        updater.add_or_update(Person::new("Person3", 20));
        updater.add_or_update(Person::new("Person4", 20));
    });

    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

#[test]
fn fires_remove_when_emptied() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    // Skip the first one, as this is setting up the stream.
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).skip(1).subscribe_next(move |updates: GroupChangeSet<Rc<Person>, String, i32>| {
        assert_eq!(updates.count(), 1, "Should be 1 update");
        for update in &updates {
            assert_eq!(update.reason(), ChangeReason::Remove);
        }
        target.set(true);
    });
    let person = Person::new("Person1", 20);

    source.edit(|updater| updater.add_or_update(person.clone()));

    // Remove.
    source.edit(|updater| updater.remove_item(&person));

    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

#[test]
fn receives_update_when_feeder_is_invoked() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).subscribe_next(move |_| target.set(true));
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 20)));
    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

#[test]
fn remove() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).skip(1).subscribe_next(move |updates: GroupChangeSet<Rc<Person>, String, i32>| {
        assert_eq!(updates.count(), 1, "Should be 1 add");
        assert_eq!(updates.changes()[0].reason(), ChangeReason::Remove);
        target.set(true);
    });
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 20)));
    source.edit(|updater| updater.remove_item(&Person::new("Person1", 20)));
    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

#[test]
fn update_an_item_will_changed_thegroup() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).subscribe_next(move |_| target.set(true));
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 20)));
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 21)));
    subscriber.dispose();
    assert!(called.get());
    source.dispose();
}

#[test]
fn update_item_after_add_item_process_add() {
    let source = new_source();
    let entries = ObservableCollectionExtended::<Rc<GroupViewModel>>::new();
    let subscriber = source.connect().group(first_letter).transform(|x: &Group<Rc<Person>, String, String>| GroupViewModel::new(x)).bind(&entries).subscribe_ignore();

    source.edit(|x| x.add_or_update(Person::new("Adam", 1)));

    let first_group = entries.get(0);
    assert_eq!(first_group.entries().count(), 1);

    source.edit(|x| {
        x.add_or_update(Person::new("Alfred", 1)); // add
        x.add_or_update(Person::new("Adam", 3)); // update
    });

    assert_eq!(first_group.entries().count(), 2);

    subscriber.dispose();
    source.dispose();
}

#[test]
fn update_not_possible() {
    let source = new_source();
    let called = Rc::new(Cell::new(false));
    let target = called.clone();
    let subscriber = source.connect().group(|p: &Rc<Person>| p.age()).skip(1).subscribe_next(move |_| target.set(true));
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 20)));
    source.edit(|updater| updater.add_or_update(Person::new("Person1", 20)));
    subscriber.dispose();
    assert!(!called.get());
    source.dispose();
}
