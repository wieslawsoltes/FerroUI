//! `FullJoinManyFixture` (`DynamicData.Tests/Cache/FullJoinManyFixture.cs`): every person and
//! every group of children, joined by the name of the parent.

use crate::domain::{ParentAndChildren, Person};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IGrouping, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::IDisposable;
use std::collections::HashMap;
use std::rc::Rc;

/// The fixture: the people and the aggregator of the join, disposed with it (`Dispose`).
struct Fixture {
    people: SourceCache<Rc<Person>, String>,
    result: ChangeSetAggregator<Rc<ParentAndChildren>, String>,
}

impl Fixture {
    fn new() -> Self {
        let people = SourceCache::new(|p: &Rc<Person>| p.name());
        let result = people
            .connect()
            .full_join_many_with_key(people.connect(), |pac: &Rc<Person>| pac.parent_name(), |person_id, person, grouping| {
                ParentAndChildren::with_parent_id(person_id.clone(), person.cloned(), grouping.items())
            })
            .as_aggregator();
        Self { people, result }
    }

    /// `AssertDataIsCorrectlyFormed`.
    fn assert_data_is_correctly_formed(&self, all_people: &[Rc<Person>]) {
        let people: HashMap<String, Rc<Person>> = all_people.iter().map(|p| (p.name(), p.clone())).collect();
        let mut all_keys: Vec<String> = Vec::new();
        for key in all_people.iter().map(|p| p.parent_name()).chain(all_people.iter().map(|p| p.name())) {
            if !all_keys.contains(&key) {
                all_keys.push(key);
            }
        }

        let all: Vec<Rc<ParentAndChildren>> = all_keys
            .iter()
            .map(|key| {
                let parent = people.get(key).cloned();
                let children: Vec<Rc<Person>> = all_people.iter().filter(|p| p.parent_name() == *key).cloned().collect();
                ParentAndChildren::with_parent_id(key.clone(), parent, children)
            })
            .collect();

        assert_eq!(self.result.data().count(), all.len());

        for parent_and_child in &all {
            let result = parent_and_child.parent_id().and_then(|parent_id| self.result.data().lookup(&parent_id)).expect("result");

            let children = result.children();
            assert_equivalent(&children, &parent_and_child.children());
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.people.dispose();
        self.result.dispose();
    }
}

/// `CalculateParent`.
fn calculate_parent(index: i32, total_people: i32) -> i32 {
    if index < 5 {
        return 11;
    }

    if index == total_people - 1 {
        return 1;
    }

    if index == total_people {
        return 1;
    }

    index + 1
}

/// The ten people `Person1` to `Person10`, with the parents of [`calculate_parent`].
fn people_with_parents() -> Vec<Rc<Person>> {
    (1..=10)
        .map(|i| {
            let parent = format!("Person{}", calculate_parent(i, 10));
            Person::with_parent_name(format!("Person{i}"), i, parent)
        })
        .collect()
}

#[test]
fn add_child() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    let person11 = Person::with_parent_name("Person11", 100, "Person3");
    fixture.people.add_or_update(person11.clone());

    let mut updated_people = people;
    updated_people.push(person11);

    fixture.assert_data_is_correctly_formed(&updated_people);
}

#[test]
fn add_left_only() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=1000).map(|i| Person::new(format!("Person{i}"), i)).collect();

    fixture.people.add_or_update_items(people.clone());
    fixture.assert_data_is_correctly_formed(&people);
}

#[test]
fn add_people_with_parents() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    fixture.assert_data_is_correctly_formed(&people);
}

#[test]
fn remove_child() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    let last = people.last().expect("ten people").clone();
    fixture.people.remove(&last);

    let updated_people: Vec<Rc<Person>> = people.iter().filter(|p| p.name() != last.name()).cloned().collect();

    fixture.assert_data_is_correctly_formed(&updated_people);
}

#[test]
fn update_child() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    let current6 = people[5].clone();
    let person6 = Person::with_parent_name("Person6", 100, current6.parent_name());
    fixture.people.add_or_update(person6.clone());

    let mut updated_people: Vec<Rc<Person>> = people.iter().filter(|p| p.name() != "Person6").cloned().collect();
    updated_people.push(person6);

    fixture.assert_data_is_correctly_formed(&updated_people);
}

#[test]
fn update_parent() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    let current10 = people.last().expect("ten people").clone();
    let person10 = Person::with_parent_name("Person10", 100, current10.parent_name());
    fixture.people.add_or_update(person10.clone());

    let mut updated_people: Vec<Rc<Person>> = people.iter().take(9).cloned().collect();
    updated_people.push(person10);

    fixture.assert_data_is_correctly_formed(&updated_people);
}
