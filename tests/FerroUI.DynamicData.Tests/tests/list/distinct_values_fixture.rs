//! `DistinctValuesFixture` (`DynamicData.Tests/List/DistinctValuesFixture.cs`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<i32>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::<Rc<Person>>::new();
        let results = source.connect().distinct_values(|p: &Rc<Person>| p.age()).as_aggregator();
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
fn adding_removed_item() {
    let fixture = Fixture::new();
    let person = Person::new("A", 20);

    fixture.source.add(person.clone());
    fixture.source.remove(&person);
    fixture.source.add(person);

    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 3, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");

    assert_equivalent(&fixture.results.data().items(), &[20]);
    assert_eq!(messages[0].adds(), 1, "First message should be an add");
    assert_eq!(messages[1].removes(), 1, "Second message should be a remove");
    assert_eq!(messages[2].adds(), 1, "Third message should be an add");
}

#[test]
fn duplicated_results_result_in_no_additional_message() {
    let fixture = Fixture::new();
    fixture.source.edit(|list| {
        list.add(Person::new("Person1", 20));
        list.add(Person::new("Person1", 20));
        list.add(Person::new("Person1", 20));
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 update message");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 items in the cache");
    assert_eq!(fixture.results.data().items()[0], 20, "Should 20");
}

#[test]
fn fires_add_whena_new_item_is_added() {
    let fixture = Fixture::new();
    fixture.source.add(Person::new("Person1", 20));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], 20, "Should 20");
}

#[test]
fn fires_batch_result_once() {
    let fixture = Fixture::new();
    fixture.source.edit(|list| {
        list.add(Person::new("Person1", 20));
        list.add(Person::new("Person2", 21));
        list.add(Person::new("Person3", 22));
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 3, "Should be 3 items in the cache");

    assert_equivalent(&fixture.results.data().items(), &[20, 21, 22]);
    assert_eq!(fixture.results.data().items()[0], 20, "Should 20");
}

#[test]
fn removing_an_item_removes_the_distinct() {
    let fixture = Fixture::new();
    let person = Person::new("Person1", 20);

    fixture.source.add(person.clone());
    fixture.source.remove(&person);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 1 update message");
    assert_eq!(fixture.results.data().count(), 0, "Should be 1 items in the cache");

    assert_eq!(messages[0].adds(), 1, "First message should be an add");
    assert_eq!(messages[1].removes(), 1, "Second messsage should be a remove");
}

#[test]
fn replacing() {
    let fixture = Fixture::new();
    let person = Person::new("A", 20);
    let replace_with = Person::new("A", 21);

    fixture.source.add(person.clone());
    fixture.source.replace(&person, replace_with);
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 1 update message");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 items in the cache");

    assert_eq!(messages[0].adds(), 1, "First message should be an add");
    assert_eq!(messages[1].count(), 2, "Second messsage should be an add an a remove");
}
