//! `FilterOnObservableFixture` (`DynamicData.Tests/List/FilterOnObservableFixture.cs`): the filter of
//! a list by a sequence of booleans of each item, here whether the sequence of the age of a
//! `PersonObs` is above 18.

use crate::domain::PersonObs;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::rc::Rc;

// Upstream's note: To optimise this, we need to introduce replace range, or specify a buffer

/// `FilterPropertyStub`: a source of people and the aggregator of
/// `FilterOnObservable(p => p.Age.Select(v => v > 18))`.
struct FilterPropertyStub {
    results: ChangeSetAggregator<Rc<PersonObs>>,
    source: SourceList<Rc<PersonObs>>,
}

impl FilterPropertyStub {
    fn new() -> Self {
        let source = SourceList::<Rc<PersonObs>>::new();
        let results = ChangeSetAggregator::new(source.connect().filter_on_observable(|p: &Rc<PersonObs>| p.age().select(|v| v > 18)));
        Self { results, source }
    }

    fn add_range(&self, people: &[Rc<PersonObs>]) {
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

fn people() -> Vec<Rc<PersonObs>> {
    (1..=100).map(|i| PersonObs::new(format!("Name{i}"), i)).collect()
}

#[test]
fn change_a_value_so_it_is_still_in_the_filter() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    people[50].set_age(100);
    assert_eq!(stub.results.data().count(), 82);
    // initial add range, refreshes to filter out < 18 and then no refresh for the no-op filter change
    //  stub.Results.Messages.Count.Should().Be(102);
}

#[test]
fn change_a_value_to_match_filter() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    people[20].set_age(10);

    // should have 100-18-1 left
    assert_eq!(stub.results.data().count(), 81);

    // initial addrange, refreshes to filter out < 18 and then refresh for the filter change
    //                stub.Results.Messages.Count.Should().Be(1+18+1);
}

#[test]
fn change_a_value_to_no_longer_match_filter() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);

    // should have 100-18 left
    assert_eq!(stub.results.data().count(), 82);

    //  stub.Results.Messages.Count.Should().Be(1+18);

    people[10].set_age(20);

    // should have 82+1 left
    assert_eq!(stub.results.data().count(), 83);

    // initial addrange, refreshes to filter out < 18 and then one refresh for the filter change
    //  stub.Results.Messages.Count.Should().Be(1+18+1);
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

    // should have 100-18 left
    assert_eq!(stub.results.data().count(), 82);

    // initial addrange, refreshes to filter out < 18
    // stub.Results.Messages.Count.Should().Be(1+18);

    assert_equivalent(&stub.results.data().items(), &people[18..]);
}

#[test]
fn remove_range() {
    let people = people();
    let stub = FilterPropertyStub::new();
    stub.add_range(&people);
    stub.remove_range(89, 10);

    assert_eq!(stub.results.data().count(), 72);
    // initial addrange, refreshes to filter out < 18 and then removerange
    //    stub.Results.Messages.Count.Should().Be(1+18+1);
}
