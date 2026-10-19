//! `ReadonlyCollectionBindCacheFixture` (`DynamicData.Tests/Binding/ReadonlyCollectionBindCacheFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{ObservableCollectionEx, ReadOnlyObservableCollection};
use ferroui_dynamic_data::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::collections::specialized::NotifyCollectionChangedAction;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

struct Fixture {
    binder: Rc<dyn IDisposable>,
    collection: ReadOnlyObservableCollection<Rc<Person>>,
    generator: RandomPersonGenerator,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let (bound, collection) = source.connect().bind_read_only();
        let binder = bound.subscribe_ignore();
        Self { binder, collection, generator: RandomPersonGenerator::new(), source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.binder.dispose();
        self.source.dispose();
    }
}

fn add_or_update(source: &SourceCache<Rc<Person>, String>, people: &[Rc<Person>]) {
    source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
}

#[test]
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));

    assert_eq!(fixture.collection.count(), 1, "Should be 1 item in the collection");
    assert_eq!(fixture.collection.get(0), person, "Should be same person");
}

#[test]
fn batch_add() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);

    assert_eq!(fixture.collection.count(), 100, "Should be 100 items in the collection");
    assert_equivalent(&fixture.collection.to_vec(), &fixture.collection.to_vec());
}

#[test]
fn batch_remove() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    add_or_update(&fixture.source, &people);
    fixture.source.edit(|updater| updater.clear());
    assert_eq!(fixture.collection.count(), 0, "Should be 100 items in the collection");
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    fixture.source.edit(|updater| updater.remove_item(&person));

    assert_eq!(fixture.collection.count(), 0, "Should be 1 item in the collection");
}

#[test]
fn update_to_source_sends_replace_on_destination() {
    let run_test = |use_replace: bool| {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let (bound, collection) = source.connect().bind_read_only_with(25, use_replace);
        let binder = bound.subscribe_ignore();

        let action = Rc::new(Cell::new(None));
        add_or_update(&source, &[Person::new("Adult1", 50)]);

        let target = action.clone();
        let subscription = collection.observe_collection_changes().select(|x| x.event_args.action()).subscribe_next(move |update_type| target.set(Some(update_type)));
        add_or_update(&source, &[Person::new("Adult1", 51)]);
        subscription.dispose();

        if use_replace {
            assert_eq!(action.get(), Some(NotifyCollectionChangedAction::Replace));
        } else {
            assert_eq!(action.get(), Some(NotifyCollectionChangedAction::Add));
        }
        binder.dispose();
        source.dispose();
    };

    run_test(true);
    run_test(false);
}

#[test]
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let person_updated = Person::new("Adult1", 51);
    add_or_update(&fixture.source, std::slice::from_ref(&person));
    add_or_update(&fixture.source, std::slice::from_ref(&person_updated));

    assert_eq!(fixture.collection.count(), 1, "Should be 1 item in the collection");
    assert!(Rc::ptr_eq(&fixture.collection.get(0), &person_updated), "Should be updated person");
}
