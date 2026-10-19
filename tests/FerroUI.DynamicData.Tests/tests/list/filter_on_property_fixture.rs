//! `FilterOnPropertyFixture` (`DynamicData.Tests/List/FilterOnPropertyFixture.cs`): the filter of a
//! list evaluated again whenever a property of an item changes (`FilterOnProperty(p => p.Age, p =>
//! p.Age > 18)`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_system::{prop, IDisposable};
use std::rc::Rc;

/// `FilterPropertyStub`: a source of people and the aggregator of
/// `FilterOnProperty(p => p.Age, p => p.Age > 18)`.
struct FilterPropertyStub {
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl FilterPropertyStub {
    fn new() -> Self {
        let source = SourceList::<Rc<Person>>::new();
        let results = ChangeSetAggregator::new(source.connect().filter_on_property(prop!(Person => age), |p: &Rc<Person>| p.age() > 18));
        Self { results, source }
    }

    fn add_range(&self, people: &[Rc<Person>]) {
        let people = people.to_vec();
        self.source.edit(|list| list.add_range(people));
    }

    fn remove_range(&self, index: usize, count: usize) {
        self.source.edit(|list| list.remove_range(index, count));
    }

    fn clear(&self) {
        self.source.edit(|list| list.clear());
    }
}

impl Drop for FilterPropertyStub {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

fn people() -> Vec<Rc<Person>> {
    (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect()
}

#[test]
fn change_a_value_so_it_is_still_in_the_filter() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    people[50].set_age(100);
    assert_eq!(stub.results.messages().len(), 2);
    assert_eq!(stub.results.data().count(), 82);
}

#[test]
fn change_a_value_to_match_filter() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    people[20].set_age(10);

    assert_eq!(stub.results.messages().len(), 2);
    assert_eq!(stub.results.data().count(), 81);
}

#[test]
fn change_a_value_to_no_longer_match_filter() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    people[10].set_age(20);

    assert_eq!(stub.results.messages().len(), 2);
    assert_eq!(stub.results.data().count(), 83);
}

#[test]
fn clear() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);
    stub.clear();

    assert_eq!(stub.results.data().count(), 0);
}

#[test]
fn initial_values() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    assert_eq!(stub.results.messages().len(), 1);
    assert_eq!(stub.results.data().count(), 82);

    assert_equivalent(&stub.results.data().items(), &people[18..]);
}

#[test]
fn remove_range() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);
    stub.remove_range(89, 10);

    assert_eq!(stub.results.data().count(), 72);
}
