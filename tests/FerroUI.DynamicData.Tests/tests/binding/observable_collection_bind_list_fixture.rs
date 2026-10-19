//! `ObservableCollectionBindListFixture` (`DynamicData.Tests/Binding/ObservableCollectionBindListFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::ObservableCollectionExtended;
use ferroui_dynamic_data::list::{replace, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    binder: Rc<dyn IDisposable>,
    collection: ObservableCollectionExtended<Rc<Person>>,
    generator: RandomPersonGenerator,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let collection = ObservableCollectionExtended::new();
        let source = SourceList::new();
        let binder = source.connect().bind(&collection).subscribe_ignore();
        Self { binder, collection, generator: RandomPersonGenerator::new(), source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.binder.dispose();
        self.source.dispose();
    }
}

#[test]
fn add_range() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));

    assert_eq!(fixture.collection.count(), 100, "Should be 100 items in the collection");
    assert_equivalent(&fixture.collection.to_vec(), &fixture.collection.to_vec());
}

#[test]
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));

    assert_eq!(fixture.collection.count(), 1, "Should be 1 item in the collection");
    assert_eq!(fixture.collection.get(0), person, "Should be same person");
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.edit(|list| list.add_range(people));
    fixture.source.edit(|list| list.clear());

    assert_eq!(fixture.collection.count(), 0, "Should be 100 items in the collection");
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));
    fixture.source.edit(|list| {
        list.remove(&person);
    });

    assert_eq!(fixture.collection.count(), 0, "Should be 1 item in the collection");
}

#[test]
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let person_updated = Person::new("Adult1", 51);
    fixture.source.edit(|list| list.add(person.clone()));
    fixture.source.edit(|list| replace(list, &person, person_updated.clone()));

    assert_eq!(fixture.collection.count(), 1, "Should be 1 item in the collection");
    assert!(Rc::ptr_eq(&fixture.collection.get(0), &person_updated), "Should be updated person");
}

