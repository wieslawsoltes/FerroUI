//! `FilterParallelFixture` (`DynamicData.Tests/Cache/FilterParallelFixture.cs`): the static filter
//! of a cache with `ParallelisationOptions(ParallelType.Ordered)`.
//!
//! The parallel overload of `Filter` runs the predicate with PLINQ, which is not ported
//! (docs/porting/reactive.md, section 5.4). `ParallelType.Ordered` keeps the order of the
//! sequential filter, and every test checks only results that the sequential filter gives as well,
//! so the tests run against the sequential [`filter`](ObservableCacheEx::filter).

use crate::domain::Person;
use crate::utilities::assert_equivalent_strict;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The fixture (`FilterParallelFixture` with its constructor and `Dispose`).
struct Fixture {
    results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        // Upstream: `Filter(p => p.Age > 20, new ParallelisationOptions(ParallelType.Ordered))`.
        let results = ChangeSetAggregator::new(source.connect().filter(|p: &Rc<Person>| p.age() > 20));
        Self { results, source }
    }

    fn add_or_update(&self, person: &Rc<Person>) {
        self.source.edit(|updater| updater.add_or_update(person.clone()));
    }

    fn add_or_update_all(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }

    fn remove_key(&self, key: &str) {
        self.source.edit(|updater| updater.remove_key(&key.to_owned()));
    }

    fn remove_all(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.remove_items(&mut people.iter().cloned()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

/// `Enumerable.Range(1, 100).Select(i => new Person("Name" + i, i))`.
fn people() -> Vec<Rc<Person>> {
    (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect()
}

#[test]
fn add_matched() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.add_or_update(&person);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], person, "Should be same person");
}

#[test]
fn add_not_matched() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 10);
    fixture.add_or_update(&person);

    assert_eq!(fixture.results.messages().len(), 0, "Should have no item updates");
    assert_eq!(fixture.results.data().count(), 0, "Cache should have no items");
}

#[test]
fn add_not_matched_and_update_matched() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let notmatched = Person::new(key, 19);
    let matched = Person::new(key, 21);

    fixture.source.edit(|updater| {
        updater.add_or_update(notmatched.clone());
        updater.add_or_update(matched.clone());
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(*fixture.results.messages()[0].changes()[0].current(), matched, "Should be same person");
    assert_eq!(fixture.results.data().items()[0], matched, "Should be same person");
}

#[test]
fn attempted_removal_of_a_non_existent_key_will_be_ignored() {
    let fixture = Fixture::new();
    let key = "Adult1";
    fixture.remove_key(key);
    assert_eq!(fixture.results.messages().len(), 0, "Should be 0 updates");
}

#[test]
fn batch_of_unique_updates() {
    let fixture = Fixture::new();
    let people = people();

    fixture.add_or_update_all(&people);
    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 80, "Should return 80 adds");

    let mut filtered: Vec<Rc<Person>> = people.iter().filter(|p| p.age() > 20).cloned().collect();
    filtered.sort_by_key(|p| p.name());
    let mut items = fixture.results.data().items();
    items.sort_by_key(|p| p.name());
    // `OrderBy(p => p.Name)` on both sides: the order is that of the names.
    assert_equivalent_strict(&items, &filtered);
}

#[test]
fn batch_removes() {
    let fixture = Fixture::new();
    let people = people();

    fixture.add_or_update_all(&people);
    fixture.remove_all(&people);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 80, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 80, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn batch_successive_updates() {
    let fixture = Fixture::new();
    let people = people();
    for person in &people {
        fixture.add_or_update(person);
    }

    assert_eq!(fixture.results.messages().len(), 80, "Should be 100 updates");
    assert_eq!(fixture.results.data().count(), 80, "Should be 100 in the cache");
    let mut filtered: Vec<Rc<Person>> = people.iter().filter(|p| p.age() > 20).cloned().collect();
    filtered.sort_by_key(|p| p.age());
    let mut items = fixture.results.data().items();
    items.sort_by_key(|p| p.age());
    // `OrderBy(p => p.Age)` on both sides: the order is that of the ages.
    assert_equivalent_strict(&items, &filtered);
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people = people();
    fixture.add_or_update_all(&people);
    fixture.source.edit(|updater| updater.clear());

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 80, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 80, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let person = Person::new(key, 50);

    fixture.add_or_update(&person);
    fixture.remove_key(key);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn same_key_changes() {
    let fixture = Fixture::new();
    let key = "Adult1";

    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new(key, 50));
        updater.add_or_update(Person::new(key, 52));
        updater.add_or_update(Person::new(key, 53));
        updater.remove_key(&key.to_owned());
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(fixture.results.messages()[0].updates(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].removes(), 1, "Should be 1 remove");
}

#[test]
fn update_matched() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let newperson = Person::new(key, 50);
    let updated = Person::new(key, 51);

    fixture.add_or_update(&newperson);
    fixture.add_or_update(&updated);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(fixture.results.messages()[1].updates(), 1, "Should be 1 update");
}

#[test]
fn update_not_matched() {
    let fixture = Fixture::new();
    let key = "Adult1";
    let newperson = Person::new(key, 10);
    let updated = Person::new(key, 11);

    fixture.add_or_update(&newperson);
    fixture.add_or_update(&updated);

    assert_eq!(fixture.results.messages().len(), 0, "Should be no updates");
    assert_eq!(fixture.results.data().count(), 0, "Should nothing cached");
}
