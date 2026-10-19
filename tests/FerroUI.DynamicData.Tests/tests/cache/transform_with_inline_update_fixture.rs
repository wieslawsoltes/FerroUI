//! `TransformWithInlineUpdateFixture` (`DynamicData.Tests/Cache/TransformWithInlineUpdateFixture.cs`):
//! the transform whose updates change the transformed item in place.

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `TransformFactory`: a copy of the person.
fn transform_factory(p: &Rc<Person>) -> Rc<Person> {
    Person::new(p.name(), p.age())
}

/// `UpdateAction`: the age of the new person.
fn update_action(transformed: &Rc<Person>, current: &Rc<Person>) {
    transformed.set_age(current.age());
}

/// `TransformWithInlineUpdateFixtureStub`.
struct Stub {
    results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Stub {
    fn new(transform_on_refresh: bool) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = ChangeSetAggregator::new(source.connect().transform_with_inline_update_with(transform_factory, update_action, transform_on_refresh));
        Self { results, source }
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn inline_update() {
    let stub = Stub::new(false);
    let person = Person::new("Adult1", 50);
    stub.add_or_update(&[person]);

    let transformed_person = stub.results.data().items()[0].clone();

    let person_update = Person::new("Adult1", 51);
    stub.add_or_update(&[person_update.clone()]);

    let updated_transform = stub.results.data().items()[0].clone();

    assert_eq!(updated_transform.age(), person_update.age(), "Age should be updated from 50 to 51.");
    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.data().count(), 1, "Should be 1 item in the cache");
    assert!(Rc::ptr_eq(&transformed_person, &stub.results.data().items()[0]), "Should be same transformed person instance.");
}

#[test]
fn batch_of_unique_updates() {
    let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();
    let stub = Stub::new(false);
    stub.add_or_update(&people);

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should return 100 adds");

    let mut transformed: Vec<Rc<Person>> = people.iter().map(transform_factory).collect();
    transformed.sort_by_key(|p| p.age());
    let mut items = stub.results.data().items();
    items.sort_by_key(|p| p.age());
    assert_equivalent(&items, &transformed);
}

#[test]
fn clear() {
    let stub = Stub::new(false);
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();

    stub.add_or_update(&people);
    stub.source.edit(|updater| updater.clear());

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should be 80 addes");
    assert_eq!(stub.results.messages()[1].removes(), 100, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn remove() {
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    let stub = Stub::new(false);
    stub.add_or_update(&[person]);
    stub.source.edit(|updater| updater.remove_key(&KEY.to_owned()));

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(stub.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn transform_on_refresh() {
    let stub = Stub::new(true);
    let person = Person::new("Adult1", 50);
    stub.add_or_update(&[person.clone()]);

    let transformed_person = stub.results.data().items()[0].clone();

    person.set_age(51);
    stub.source.edit(|updater| updater.refresh_item(&person));

    let updated_transform = stub.results.data().items()[0].clone();

    assert_eq!(updated_transform.age(), 51, "Age should be updated from 50 to 51.");
    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.data().count(), 1, "Should be 1 item in the cache");
    assert!(Rc::ptr_eq(&transformed_person, &stub.results.data().items()[0]), "Should be same transformed person instance.");
}
