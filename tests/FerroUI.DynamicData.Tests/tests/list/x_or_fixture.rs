//! `XOrFixture` (`DynamicData.Tests/List/XOrFixture.cs`): the items of two lists that are in
//! exactly one, from a sequence with another (`XOrFixture`) and from a collection of sequences
//! (`XOrCollectionFixture`). The abstract `XOrFixtureBase` is the functions of the tests, which
//! each fixture runs with its `CreateObservable`.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ListSourcesCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// `CreateObservable`.
type CreateObservable = fn(&SourceList<i32>, &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>>;

/// `XOrFixture.CreateObservable`: `_source1.Connect().Xor(_source2.Connect())`.
fn create_with_others(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    source1.connect().xor(vec![source2.connect()])
}

/// `XOrCollectionFixture.CreateObservable`: `new List { .. }.Xor()`.
fn create_collection(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    let l = vec![source1.connect(), source2.connect()];
    l.xor()
}

/// `XOrFixtureBase`: the two sources and the results.
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

fn clear_only_clears_one_source(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..6);
    f.source2.add_range(6..11);
    f.source1.clear();
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &(6..11).collect::<Vec<_>>());
}

fn combine_range(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..6);
    f.source2.add_range(6..11);
    assert_eq!(f.results.data().count(), 10);
    assert_equivalent(&f.results.data().items(), &(1..11).collect::<Vec<_>>());
}

fn included_when_item_is_in_one_source(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    assert_eq!(f.results.data().count(), 1);
    assert_eq!(f.results.data().items()[0], 1);
}

fn not_included_when_item_is_in_two_sources(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 0);
}

fn overlapping_range_exludes_interesct(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add_range(1..11);
    f.source2.add_range(6..16);
    assert_eq!(f.results.data().count(), 10);
    assert_equivalent(&f.results.data().items(), &(1..6).chain(11..16).collect::<Vec<_>>());
}

fn removed_when_no_longer_in_both(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    f.source1.remove(&1);
    assert_eq!(f.results.data().count(), 1);
}

fn removed_when_no_longer_in_either(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source1.remove(&1);
    assert_eq!(f.results.data().count(), 0);
}
/// The `[Fact]`s of `XOrFixtureBase`, for the fixture of `$create_observable`.
macro_rules! fixture_base_tests {
    ($create_observable:path) => {
        #[test]
        fn clear_only_clears_one_source() {
            super::clear_only_clears_one_source($create_observable);
        }

        #[test]
        fn combine_range() {
            super::combine_range($create_observable);
        }

        #[test]
        fn included_when_item_is_in_one_source() {
            super::included_when_item_is_in_one_source($create_observable);
        }

        #[test]
        fn not_included_when_item_is_in_two_sources() {
            super::not_included_when_item_is_in_two_sources($create_observable);
        }

        #[test]
        fn overlapping_range_exludes_interesct() {
            super::overlapping_range_exludes_interesct($create_observable);
        }

        #[test]
        fn removed_when_no_longer_in_both() {
            super::removed_when_no_longer_in_both($create_observable);
        }

        #[test]
        fn removed_when_no_longer_in_either() {
            super::removed_when_no_longer_in_either($create_observable);
        }
    };
}

/// `XOrFixture`.
mod with_others {
    fixture_base_tests!(super::create_with_others);
}

/// `XOrCollectionFixture`.
mod collection {
    fixture_base_tests!(super::create_collection);
}
