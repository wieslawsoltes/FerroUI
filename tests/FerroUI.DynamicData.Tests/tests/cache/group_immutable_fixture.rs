//! `GroupImmutableFixture` (`DynamicData.Tests/Cache/GroupImmutableFixture.cs`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeReason, IConnectableCache, IGrouping, IObservableCache, ImmutableGroup, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::collections::HashMap;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<ImmutableGroup<Rc<Person>, String, i32>, i32>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = source.connect().group_with_immutable_state(|p: &Rc<Person>| p.age()).as_aggregator();
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

/// `GroupBy(p => p.Age)`.
fn group_by_age(people: &[Rc<Person>]) -> HashMap<i32, Vec<Rc<Person>>> {
    let mut groups: HashMap<i32, Vec<Rc<Person>>> = HashMap::new();
    for person in people {
        groups.entry(person.age()).or_default().push(person.clone());
    }
    groups
}

#[test]
fn add() {
    let fixture = Fixture::new();
    fixture.add_or_update(vec![Person::new("Person1", 20)]);
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 add");
    assert_eq!(fixture.results.messages()[0].adds(), 1);
}

#[test]
fn chaneg_multiple_groups() {
    let fixture = Fixture::new();
    let initial_people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i % 10)).collect();

    fixture.add_or_update(initial_people.clone());

    for (key, group) in group_by_age(&initial_people) {
        let cache = fixture.results.data().lookup(&key).expect("the group is there");
        assert_equivalent(&cache.items(), &group);
    }

    let changed_people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), i % 5)).collect();

    fixture.add_or_update(changed_people.clone());

    for (key, group) in group_by_age(&changed_people) {
        let cache = fixture.results.data().lookup(&key).expect("the group is there");
        assert_equivalent(&cache.items(), &group);
    }

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), 10);
    assert_eq!(messages[1].removes(), 5);
    assert_eq!(messages[1].updates(), 5);
}

#[test]
fn fires_many_value_for_batch_of_different_adds() {
    let fixture = Fixture::new();
    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person2", 21));
        updater.add_or_update(Person::new("Person3", 22));
        updater.add_or_update(Person::new("Person4", 23));
    });

    assert_eq!(fixture.results.data().count(), 4);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].count(), 4);
    for update in &messages[0] {
        assert_eq!(update.reason(), ChangeReason::Add);
    }
}

#[test]
fn fires_only_once_for_a_batch_of_unique_values() {
    let fixture = Fixture::new();
    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person2", 20));
        updater.add_or_update(Person::new("Person3", 20));
        updater.add_or_update(Person::new("Person4", 20));
    });

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(fixture.results.data().items()[0].count(), 4);
}

#[test]
fn reevaluate() {
    let fixture = Fixture::new();
    let initial_people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("Person{i}"), i % 2)).collect();

    fixture.add_or_update(initial_people.clone());
    assert_eq!(fixture.results.messages().len(), 1);

    // Do an inline update.
    for person in &initial_people {
        person.set_age(person.age() + 1);
    }

    // Signal the operators to evaluate again.
    fixture.source.edit(|updater| updater.refresh_all());

    for (key, group) in group_by_age(&initial_people) {
        let cache = fixture.results.data().lookup(&key).expect("the group is there");
        assert_equivalent(&cache.items(), &group);
    }

    assert_eq!(fixture.results.data().count(), 2);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2);

    let second_message = &messages[1];
    assert_eq!(second_message.removes(), 1);
    assert_eq!(second_message.updates(), 1);
    assert_eq!(second_message.adds(), 1);
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    fixture.add_or_update(vec![Person::new("Person1", 20)]);
    fixture.source.edit(|updater| updater.remove_item(&Person::new("Person1", 20)));

    assert_eq!(fixture.results.messages().len(), 2);
    assert_eq!(fixture.results.data().count(), 0);
}

#[test]
fn update_an_item_will_changed_thegroup() {
    let fixture = Fixture::new();
    fixture.add_or_update(vec![Person::new("Person1", 20)]);
    fixture.add_or_update(vec![Person::new("Person1", 21)]);

    assert_eq!(fixture.results.data().count(), 1);
    let messages = fixture.results.messages();
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(messages[1].adds(), 1);
    assert_eq!(messages[1].removes(), 1);
    let group = fixture.results.data().items()[0].clone();
    assert_eq!(group.count(), 1);

    assert_eq!(*group.key(), 21);
}

#[test]
fn updates_are_permissible() {
    let fixture = Fixture::new();
    fixture.add_or_update(vec![Person::new("Person1", 20)]);
    fixture.add_or_update(vec![Person::new("Person2", 20)]);

    assert_eq!(fixture.results.data().count(), 1); // 1 group
    let messages = fixture.results.messages();
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(messages[1].updates(), 1);

    let group = fixture.results.data().items()[0].clone();
    assert_eq!(group.count(), 2);
}
