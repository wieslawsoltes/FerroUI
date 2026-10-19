//! `GroupOnFixture` (`DynamicData.Tests/List/GroupOnFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{Group, IObservableList, ObservableListEx, SourceList};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Group<Rc<Person>, i32>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let results = source.connect().group_on(|p: &Rc<Person>| p.age()).as_aggregator();
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn add() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");

    let first_group = fixture.results.data().items()[0].list().items();
    assert_eq!(first_group[0], person, "Should be same person");
}

#[test]
fn big_list() {
    let fixture = Fixture::new();
    let generator = RandomPersonGenerator::new();
    let people = generator.take(10000);
    fixture.source.edit(|list| list.add_range(people));
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.edit(|list| list.add(person.clone()));
    fixture.source.edit(|list| {
        list.remove(&person);
    });
    assert_eq!(fixture.results.messages().len(), 2, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 0, "Should be no groups");
}

#[test]
fn update_will_change_the_group() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    let amended = Person::new("Adult1", 60);
    fixture.source.edit(|list| list.add(person.clone()));
    fixture.source.edit(|list| list.set(0, amended.clone()));

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");

    let first_group = fixture.results.data().items()[0].list().items();
    assert_eq!(first_group[0], amended, "Should be same person");
}
