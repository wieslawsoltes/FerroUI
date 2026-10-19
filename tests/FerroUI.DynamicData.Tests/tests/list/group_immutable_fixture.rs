//! `GroupImmutableFixture` (`DynamicData.Tests/List/GroupImmutableFixture.cs`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{remove_many, replace, IGrouping, IObservableList, ImmutableGroup, ListChangeReason, ObservableListEx, SourceList};
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObserver};
use std::collections::HashMap;
use std::rc::Rc;

struct Fixture {
    regrouper: Rc<Subject<()>>,
    results: ChangeSetAggregator<ImmutableGroup<Rc<Person>, i32>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let regrouper = Subject::new();
        let results = source.connect().group_with_immutable_state_with(|p: &Rc<Person>| p.age(), regrouper.as_observable()).as_aggregator();
        Self { regrouper, results, source }
    }

    fn add(&self, person: Rc<Person>) {
        self.source.edit(|list| list.add(person));
    }

    /// The state of the group of `key` in the results.
    fn group(&self, key: i32) -> ImmutableGroup<Rc<Person>, i32> {
        self.results.data().items().into_iter().find(|g| *g.key() == key).expect("the group is there")
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
    fixture.add(Person::new("Person1", 20));
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 add");
    assert_eq!(fixture.results.messages()[0].adds(), 1);
}

#[test]
fn chaneg_multiple_groups() {
    let fixture = Fixture::new();
    let initial_people: Vec<Rc<Person>> = (1..=10000).map(|i| Person::new(format!("Person{i}"), i % 10)).collect();

    let added = initial_people.clone();
    fixture.source.edit(|list| list.add_range(added));

    for (key, group) in group_by_age(&initial_people) {
        let grp = fixture.group(key);
        assert_equivalent(&grp.items(), &group);
    }

    let removed: Vec<Rc<Person>> = initial_people.iter().take(15).cloned().collect();
    fixture.source.edit(|list| remove_many(list, &removed));

    for (key, group) in group_by_age(&initial_people[15..]) {
        let list = fixture.group(key);
        assert_equivalent(&list.items(), &group);
    }

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].adds(), 10);
    assert_eq!(messages[1].replaced(), 10);
}

#[test]
fn fires_many_value_for_batch_of_different_adds() {
    let fixture = Fixture::new();
    fixture.source.edit(|updater| {
        updater.add(Person::new("Person1", 20));
        updater.add(Person::new("Person2", 21));
        updater.add(Person::new("Person3", 22));
        updater.add(Person::new("Person4", 23));
    });

    assert_eq!(fixture.results.data().count(), 4);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].count(), 1);
    for update in &messages[0] {
        assert_eq!(update.reason(), ListChangeReason::AddRange);
    }
}

#[test]
fn fires_only_once_for_a_batch_of_unique_values() {
    let fixture = Fixture::new();
    fixture.source.edit(|updater| {
        updater.add(Person::new("Person1", 20));
        updater.add(Person::new("Person2", 20));
        updater.add(Person::new("Person3", 20));
        updater.add(Person::new("Person4", 20));
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

    let added = initial_people.clone();
    fixture.source.edit(|list| list.add_range(added));
    assert_eq!(fixture.results.messages().len(), 1);

    // Do an inline update.
    for person in &initial_people {
        person.set_age(person.age() + 1);
    }

    // Signal the operators to evaluate again.
    fixture.regrouper.on_next(());

    for (key, group_container) in group_by_age(&initial_people) {
        let grouping = fixture.group(key);
        assert_equivalent(&grouping.items(), &group_container);
    }

    assert_eq!(fixture.results.data().count(), 2);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2);

    let second_message = &messages[1];
    assert_eq!(second_message.removes(), 1);
    assert_eq!(second_message.replaced(), 1);
    assert_eq!(second_message.adds(), 1);
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let person = Person::new("Person1", 20);
    fixture.add(person.clone());
    fixture.source.edit(|list| {
        list.remove(&person);
    });

    assert_eq!(fixture.results.messages().len(), 2);
    assert_eq!(fixture.results.data().count(), 0);
}

#[test]
fn update_an_item_will_changed_thegroup() {
    let fixture = Fixture::new();
    let person1 = Person::new("Person1", 20);
    fixture.add(person1.clone());
    fixture.source.edit(|list| replace(list, &person1, Person::new("Person1", 21)));

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
    fixture.add(Person::new("Person1", 20));
    fixture.add(Person::new("Person2", 20));

    assert_eq!(fixture.results.data().count(), 1); // 1 group
    let messages = fixture.results.messages();
    assert_eq!(messages[0].adds(), 1);
    assert_eq!(messages[1].replaced(), 1);

    let group = fixture.results.data().items()[0].clone();
    assert_eq!(group.count(), 2);
}
