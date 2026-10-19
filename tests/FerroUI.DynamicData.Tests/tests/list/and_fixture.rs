//! `AndFixture` (`DynamicData.Tests/List/AndFixture.cs`): the items of two lists that are in
//! both, from a sequence with another (`AndFixture`) and from a collection of sequences
//! (`AndCollectionFixture`). The abstract `AndFixtureBase` is the functions of the tests, which
//! each fixture runs with its `CreateObservable`.
//!
//! Naming (DEVIATIONS.md, DynamicData): `And` is `and_list`.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ListSourcesCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// `CreateObservable`.
type CreateObservable = fn(&SourceList<i32>, &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>>;

/// `AndFixture.CreateObservable`: `_source1.Connect().And(_source2.Connect())`.
fn create_with_others(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    source1.connect().and_list(vec![source2.connect()])
}

/// `AndCollectionFixture.CreateObservable`: `new List { .. }.And()`.
fn create_collection(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    let l = vec![source1.connect(), source2.connect()];
    l.and_list()
}

/// `AndFixtureBase`: the two sources and the results.
struct Fixture {
    source1: SourceList<i32>,
    source2: SourceList<i32>,
    results: ChangeSetAggregator<i32>,
    create_observable: CreateObservable,
}

impl Fixture {
    fn new(create_observable: CreateObservable) -> Self {
        let source1 = SourceList::new();
        let source2 = SourceList::new();
        let results = create_observable(&source1, &source2).as_aggregator();
        Self { source1, source2, results, create_observable }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source1.dispose();
        self.source2.dispose();
        self.results.dispose();
    }
}

fn clear_one_clears_result(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..6);
    f.source2.add_range(1..6);
    f.source1.clear();
    assert_eq!(f.results.data().count(), 0);
}

fn combine_range(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..11);
    f.source2.add_range(6..16);
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &(6..11).collect::<Vec<_>>());
}

fn excluded_when_item_is_in_one_source(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    assert_eq!(f.results.data().count(), 0);
}

fn included_when_item_is_in_two_sources(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 1);
}

fn removed_when_no_longer_in_both(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    f.source1.remove(&1);
    assert_eq!(f.results.data().count(), 0);
}

fn starting_with_non_empty_source_produces_no_result(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);

    let result = (f.create_observable)(&f.source1, &f.source2).as_aggregator();
    assert_eq!(result.data().count(), 0);
    result.dispose();
}
/// The `[Fact]`s of `AndFixtureBase`, for the fixture of `$create_observable`.
macro_rules! fixture_base_tests {
    ($create_observable:path) => {
        #[test]
        fn clear_one_clears_result() {
            super::clear_one_clears_result($create_observable);
        }

        #[test]
        fn combine_range() {
            super::combine_range($create_observable);
        }

        #[test]
        fn excluded_when_item_is_in_one_source() {
            super::excluded_when_item_is_in_one_source($create_observable);
        }

        #[test]
        fn included_when_item_is_in_two_sources() {
            super::included_when_item_is_in_two_sources($create_observable);
        }

        #[test]
        fn removed_when_no_longer_in_both() {
            super::removed_when_no_longer_in_both($create_observable);
        }

        #[test]
        fn starting_with_non_empty_source_produces_no_result() {
            super::starting_with_non_empty_source_produces_no_result($create_observable);
        }
    };
}

/// `AndFixture`.
mod with_others {
    fixture_base_tests!(super::create_with_others);
}

/// `AndCollectionFixture`.
mod collection {
    fixture_base_tests!(super::create_collection);
}
