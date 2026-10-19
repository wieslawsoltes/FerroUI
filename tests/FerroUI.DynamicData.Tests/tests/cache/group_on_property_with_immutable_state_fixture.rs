//! `GroupOnPropertyWithImmutableStateFixture`
//! (`DynamicData.Tests/Cache/GroupOnPropertyWithImmutableStateFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IGrouping, IObservableCache, ImmutableGroup, ObservableCacheEx, SourceCache};
use ferroui_system::{prop, IDisposable};
use std::collections::HashSet;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<ImmutableGroup<Rc<Person>, String, i32>, i32>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = source.connect().group_on_property_with_immutable_state(prop!(Person => age)).as_aggregator();
        Self { results, source }
    }

    fn add_or_update(&self, people: Vec<Rc<Person>>) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.into_iter()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

fn distinct_ages(people: &[Rc<Person>]) -> usize {
    people.iter().map(|p| p.age()).collect::<HashSet<_>>().len()
}

#[test]
fn can_group_on_adds() {
    let fixture = Fixture::new();
    fixture.add_or_update(vec![Person::new("A", 10)]);

    assert_eq!(fixture.results.data().count(), 1);

    let first_group = fixture.results.data().items()[0].clone();

    assert_eq!(first_group.count(), 1);
    assert_eq!(*first_group.key(), 10);
}

#[test]
fn can_handle_add_batch() {
    let fixture = Fixture::new();
    let generator = RandomPersonGenerator::new();
    let people = generator.take(1000);

    fixture.add_or_update(people.clone());

    let expected_group_count = distinct_ages(&people);
    assert_eq!(fixture.results.data().count(), expected_group_count);
}

#[test]
fn can_handle_changed_items_batch() {
    let fixture = Fixture::new();
    let generator = RandomPersonGenerator::new();
    let people = generator.take(100);

    fixture.add_or_update(people.clone());

    let initial_count = distinct_ages(&people);
    assert_eq!(fixture.results.data().count(), initial_count);

    for person in people.iter().take(25) {
        person.set_age(200);
    }

    let changed_count = distinct_ages(&people);
    assert_eq!(fixture.results.data().count(), changed_count);

    // Each item is in one group only.
    let people_in_cache: Vec<Rc<Person>> = fixture.results.data().items().iter().flat_map(|g| g.items()).collect();

    assert_eq!(people_in_cache.len(), 100);
}

#[test]
fn can_remove_from_group() {
    let fixture = Fixture::new();
    let person = Person::new("A", 10);
    fixture.add_or_update(vec![person.clone()]);
    fixture.source.edit(|updater| updater.remove_item(&person));

    assert_eq!(fixture.results.data().count(), 0);
}

#[test]
fn regroup() {
    let fixture = Fixture::new();
    let person = Person::new("A", 10);
    fixture.add_or_update(vec![person.clone()]);
    person.set_age(20);

    assert_eq!(fixture.results.data().count(), 1);
    let first_group = fixture.results.data().items()[0].clone();

    assert_eq!(first_group.count(), 1);
    assert_eq!(*first_group.key(), 20);
}
