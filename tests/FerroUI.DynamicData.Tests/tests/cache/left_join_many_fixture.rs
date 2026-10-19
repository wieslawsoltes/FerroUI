//! `LeftJoinManyFixture` (`DynamicData.Tests/Cache/LeftJoinManyFixture.cs`): every person joined
//! to the group of their children, which may be empty.

use crate::domain::{ParentAndChildren, Person};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IGrouping, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::IDisposable;
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
            .left_join_many(people.connect(), |pac: &Rc<Person>| pac.parent_name(), |person, grouping| ParentAndChildren::new(person.clone(), grouping.items()))
            .as_aggregator();
        Self { people, result }
    }

    /// The pairs of the names of each parent and of each of their children.
    fn pairs(&self) -> Vec<(Option<String>, String)> {
        self.result
            .data()
            .items()
            .iter()
            .flat_map(|family| {
                let parent_name = family.parent().map(|parent| parent.name());
                family.children().into_iter().map(move |child| (parent_name.clone(), child.name()))
            })
            .collect()
    }

    /// `AssertDataIsCorrectlyFormed`.
    fn assert_data_is_correctly_formed(&self, expected: &[Rc<Person>], missing_parents: &[&str]) {
        assert_eq!(self.result.data().count(), expected.len());
        let parents: Vec<Rc<Person>> = self.result.data().items().iter().map(|pac| pac.parent().expect("a left join has the parent")).collect();
        assert_equivalent(&parents, expected);

        for (key, grouping) in group_by_parent_name(expected) {
            if !missing_parents.is_empty() && missing_parents.contains(&key.as_str()) {
                continue;
            }

            let result = self.result.data().lookup(&key).unwrap_or_else(|| panic!("Missing result for {key}"));

            let children = result.children();
            assert_equivalent(&children, &grouping);
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.people.dispose();
        self.result.dispose();
    }
}

/// `GroupBy(p => p.ParentName)`, in the order of the first person of each group.
fn group_by_parent_name(people: &[Rc<Person>]) -> Vec<(String, Vec<Rc<Person>>)> {
    let mut groups: Vec<(String, Vec<Rc<Person>>)> = Vec::new();
    for person in people {
        let parent_name = person.parent_name();
        match groups.iter_mut().find(|(key, _)| *key == parent_name) {
            Some((_, members)) => members.push(person.clone()),
            None => groups.push((parent_name, vec![person.clone()])),
        }
    }
    groups
}

/// `CalculateParent`.
fn calculate_parent(index: i32, total_people: i32) -> i32 {
    if index < 5 {
        return 10;
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

/// `new Person() { Name = name, ParentName = parentName }`.
fn named_person(name: &str, parent_name: &str) -> Rc<Person> {
    Person::with_all(name, 0, "none", parent_name)
}

fn pair(parent_name: &str, child_name: &str) -> (Option<String>, String) {
    (Some(parent_name.to_string()), child_name.to_string())
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

    fixture.assert_data_is_correctly_formed(&updated_people, &[]);
}

#[test]
fn add_left_only() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("Person{i}"), i)).collect();

    fixture.people.add_or_update_items(people.clone());

    assert_eq!(fixture.result.data().count(), 10);
    let parents: Vec<Rc<Person>> = fixture.result.data().items().iter().map(|pac| pac.parent().expect("a left join has the parent")).collect();
    assert_equivalent(&parents, &people);

    for pac in fixture.result.data().items() {
        assert_eq!(pac.count(), 0);
    }
}

#[test]
fn add_people_with_parents() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    fixture.assert_data_is_correctly_formed(&people, &[]);
}

#[test]
fn refresh_right_key() {
    let fixture = Fixture::new();
    fixture.people.edit(|inner_cache| {
        inner_cache.add_or_update(named_person("Person #1", ""));
        inner_cache.add_or_update(named_person("Person #2", "Person #1"));
        inner_cache.add_or_update(named_person("Person #3", "Person #2"));
    });

    let refresh_person = fixture.people.lookup(&"Person #2".to_string()).expect("Person #2");

    // Change pairing
    refresh_person.set_parent_name("Person #3");
    fixture.people.refresh(&refresh_person);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair("Person #1", "Person #2")));
    assert!(fixture.pairs().contains(&pair("Person #3", "Person #2")));

    // Remove pairing
    refresh_person.set_parent_name("Person #4");
    fixture.people.refresh(&refresh_person);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().iter().any(|(_, child_name)| child_name == "Person #2"));

    // Restore pairing
    refresh_person.set_parent_name("Person #1");
    fixture.people.refresh(&refresh_person);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.pairs().contains(&pair("Person #1", "Person #2")));

    // No change
    fixture.people.refresh(&refresh_person);

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.pairs().contains(&pair("Person #1", "Person #2")));
}

#[test]
fn remove_child() {
    let fixture = Fixture::new();
    let people = people_with_parents();

    fixture.people.add_or_update_items(people.clone());

    let last = people.last().expect("ten people").clone();
    fixture.people.remove(&last);

    let updated_people: Vec<Rc<Person>> = people.iter().filter(|p| p.name() != last.name()).cloned().collect();

    fixture.assert_data_is_correctly_formed(&updated_people, &[last.name().as_str()]);
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

    fixture.assert_data_is_correctly_formed(&updated_people, &[]);
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

    fixture.assert_data_is_correctly_formed(&updated_people, &[]);
}

#[test]
fn update_right_key() {
    let fixture = Fixture::new();
    fixture.people.edit(|inner_cache| {
        inner_cache.add_or_update(named_person("Person #1", ""));
        inner_cache.add_or_update(named_person("Person #2", "Person #1"));
        inner_cache.add_or_update(named_person("Person #3", "Person #2"));
    });

    // Change pairing
    fixture.people.add_or_update(named_person("Person #2", "Person #3"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().contains(&pair("Person #1", "Person #2")));
    assert!(fixture.pairs().contains(&pair("Person #3", "Person #2")));

    // Remove pairing
    fixture.people.add_or_update(named_person("Person #2", "Person #4"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(!fixture.pairs().iter().any(|(_, child_name)| child_name == "Person #2"));

    // Restore pairing
    fixture.people.add_or_update(named_person("Person #2", "Person #1"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.pairs().contains(&pair("Person #1", "Person #2")));

    // No change
    fixture.people.add_or_update(named_person("Person #2", "Person #1"));

    assert_eq!(fixture.result.data().count(), 3);
    assert!(fixture.pairs().contains(&pair("Person #1", "Person #2")));
}
