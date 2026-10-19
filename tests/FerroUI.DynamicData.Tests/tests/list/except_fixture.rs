//! `ExceptFixture` (`DynamicData.Tests/List/ExceptFixture.cs`): the items of a list that are not in
//! another, from a sequence with another (`ExceptFixture`) and from a collection of sequences
//! (`ExceptCollectionFixture`). The abstract `ExceptFixtureBase` is the functions of the tests,
//! which each fixture runs with its `CreateObservable`.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ListSourcesCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// `CreateObservable`.
type CreateObservable = fn(&SourceList<i32>, &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>>;

/// `ExceptFixture.CreateObservable`: `_source1.Connect().Except(_source2.Connect())`.
fn create_with_others(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    source1.connect().except(vec![source2.connect()])
}

/// `ExceptCollectionFixture.CreateObservable`: `new List { .. }.Except()`.
fn create_collection(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    let l = vec![source1.connect(), source2.connect()];
    l.except()
}

/// `ExceptFixtureBase`: the two sources and the results.
struct Fixture {
    source1: SourceList<i32>,
    source2: SourceList<i32>,
    results: ChangeSetAggregator<i32>,
}

impl Fixture {
    fn new(create_observable: CreateObservable) -> Self {
        let source1 = SourceList::new();
        let source2 = SourceList::new();
        let results = create_observable(&source1, &source2).as_aggregator();
        Self { source1, source2, results }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source1.dispose();
        self.source2.dispose();
        self.results.dispose();
    }
}

fn added_when_no_longer_in_second(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    f.source2.remove(&1);
    assert_eq!(f.results.data().count(), 1);
}

fn clear_first_clears_result(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..6);
    f.source2.add_range(1..6);
    f.source1.clear();
    assert_eq!(f.results.data().count(), 0);
}

fn clear_second_ensures_first_is_included(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..6);
    f.source2.add_range(1..6);
    assert_eq!(f.results.data().count(), 0);
    f.source2.clear();
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &(1..6).collect::<Vec<_>>());
}

fn combine_range(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..11);
    f.source2.add_range(6..16);
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &(1..6).collect::<Vec<_>>());
}

fn excluded_when_item_is_in_two_sources(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 0);
}

fn included_when_item_is_in_one_source(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    assert_eq!(f.results.data().count(), 1);
}

fn nothing_from_other(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 0);
}
/// The `[Fact]`s of `ExceptFixtureBase`, for the fixture of `$create_observable`.
macro_rules! fixture_base_tests {
    ($create_observable:path) => {
        #[test]
        fn added_when_no_longer_in_second() {
            super::added_when_no_longer_in_second($create_observable);
        }

        #[test]
        fn clear_first_clears_result() {
            super::clear_first_clears_result($create_observable);
        }

        #[test]
        fn clear_second_ensures_first_is_included() {
            super::clear_second_ensures_first_is_included($create_observable);
        }

        #[test]
        fn combine_range() {
            super::combine_range($create_observable);
        }

        #[test]
        fn excluded_when_item_is_in_two_sources() {
            super::excluded_when_item_is_in_two_sources($create_observable);
        }

        #[test]
        fn included_when_item_is_in_one_source() {
            super::included_when_item_is_in_one_source($create_observable);
        }

        #[test]
        fn nothing_from_other() {
            super::nothing_from_other($create_observable);
        }
    };
}

/// `ExceptFixture`.
mod with_others {
    fixture_base_tests!(super::create_with_others);
}

/// `ExceptCollectionFixture`.
mod collection {
    fixture_base_tests!(super::create_collection);
}
