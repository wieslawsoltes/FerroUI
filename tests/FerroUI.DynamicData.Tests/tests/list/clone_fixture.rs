//! `CloneFixture` (`DynamicData.Tests/List/CloneFixture.cs`): upstream's fixture of the list folder
//! tests the `Clone` of caches, into a collection.

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

struct Fixture {
    cloner: Rc<dyn IDisposable>,
    collection: Rc<RefCell<Vec<Rc<Person>>>>,
    generator: RandomPersonGenerator,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let collection = Rc::new(RefCell::new(Vec::new()));
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let cloner = source.connect().clone_to(collection.clone()).subscribe_ignore();
        Self { cloner, collection, generator: RandomPersonGenerator::new(), source }
    }

    fn collection(&self) -> Vec<Rc<Person>> {
        self.collection.borrow().clone()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.cloner.dispose();
        self.source.dispose();
    }
}

#[test]
fn add_to_source_adds_to_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.add_or_update(person.clone());

    let collection = fixture.collection();
    assert_eq!(collection.len(), 1, "Should be 1 item in the collection");
    assert_eq!(collection[0], person, "Should be same person");
}

#[test]
fn batch_add() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.add_or_update_items(people);

    let collection = fixture.collection();
    assert_eq!(collection.len(), 100, "Should be 100 items in the collection");
    // Upstream compares the collection with itself.
    assert_equivalent(&collection, &collection);
}

#[test]
fn batch_remove() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.source.add_or_update_items(people);
    fixture.source.clear();
    assert_eq!(fixture.collection().len(), 0, "Should be 100 items in the collection");
}

#[test]
fn remove_source_removes_from_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.add_or_update(person.clone());
    fixture.source.remove(&person);

    assert_eq!(fixture.collection().len(), 0, "Should be 1 item in the collection");
}

#[test]
fn update_to_source_updates_the_destination() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let person_updated = Person::new("Adult1", 51);
    fixture.source.add_or_update(person);
    fixture.source.add_or_update(person_updated.clone());

    let collection = fixture.collection();
    assert_eq!(collection.len(), 1, "Should be 1 item in the collection");
    assert_eq!(collection[0], person_updated, "Should be updated person");
}
