//! `ReverseFixture` (`DynamicData.Tests/List/ReverseFixture.cs`).

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IExtendedList, IObservableList, ObservableListEx, SourceList};
use ferroui_system::IDisposable;

struct Fixture {
    results: ChangeSetAggregator<i32>,
    source: SourceList<i32>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let results = source.connect().reverse().as_aggregator();
        Self { results, source }
    }

    fn add_range(&self) {
        self.source.edit(|list| list.add_range((1..=5).collect()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
    }
}

#[test]
fn add_in_sucession() {
    let fixture = Fixture::new();
    for item in 1..=5 {
        fixture.source.edit(|list| list.add(item));
    }

    assert_equivalent(&fixture.results.data().items(), &[5, 4, 3, 2, 1]);
}

#[test]
fn add_range() {
    let fixture = Fixture::new();
    fixture.add_range();

    assert_equivalent(&fixture.results.data().items(), &[5, 4, 3, 2, 1]);
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| list.clear());

    assert_eq!(fixture.results.data().count(), 0);
}

#[test]
fn move_() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| IExtendedList::move_index(list, 4, 1));

    assert_equivalent(&fixture.results.data().items(), &[4, 3, 2, 5, 1]);
}

#[test]
fn move2() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| IExtendedList::move_index(list, 1, 4));

    assert_equivalent(&fixture.results.data().items(), &[2, 5, 4, 3, 1]);
}

#[test]
fn remove_range() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| list.remove_range(1, 3));

    assert_equivalent(&fixture.results.data().items(), &[5, 1]);
}

#[test]
fn remove_range_then_insert() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| list.remove_range(1, 3));
    fixture.source.edit(|list| list.insert(1, 3));

    assert_equivalent(&fixture.results.data().items(), &[5, 3, 1]);
}

#[test]
fn removes() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| {
        list.remove(&1);
    });
    fixture.source.edit(|list| {
        list.remove(&4);
    });

    assert_equivalent(&fixture.results.data().items(), &[5, 3, 2]);
}

#[test]
fn replace() {
    let fixture = Fixture::new();
    fixture.add_range();
    fixture.source.edit(|list| list.set(2, 100));

    assert_equivalent(&fixture.results.data().items(), &[5, 4, 100, 2, 1]);
}
