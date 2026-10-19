//! `DynamicAndFixture` (`DynamicData.Tests/List/DynamicAndFixture.cs`): the items of the lists of a
//! list that are in all of them, as lists are added to the list and removed from it
//! (`And(IObservableList<IObservable<IChangeSet<T>>>)`).
//!
//! The list holds sequences, which compare by identity (`SourceList<IObservable<..>>`).

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ListSourceListCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// The change sets of a list of numbers (`IObservable<IChangeSet<int>>`).
type NumberChanges = Rc<dyn IObservable<ChangeSet<i32>>>;

/// The fixture: the list of sequences, the three lists and the results.
struct Fixture {
    results: ChangeSetAggregator<i32>,
    source: SourceList<NumberChanges>,
    source1: SourceList<i32>,
    source2: SourceList<i32>,
    source3: SourceList<i32>,
}

impl Fixture {
    fn new() -> Self {
        let source1 = SourceList::new();
        let source2 = SourceList::new();
        let source3 = SourceList::new();
        let source: SourceList<NumberChanges> = SourceList::new();
        let results = source.and_list().as_aggregator();
        Self { results, source, source1, source2, source3 }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source1.dispose();
        self.source2.dispose();
        self.source3.dispose();
        self.source.dispose();
        self.results.dispose();
    }
}

/// `Enumerable.Range(start, count)`.
fn range(start: i32, count: i32) -> Vec<i32> {
    (start..start + count).collect()
}

#[test]
fn add_and_remove_lists() {
    let f = Fixture::new();
    f.source1.add_range(range(1, 5));
    f.source3.add_range(range(1, 5));

    f.source.add(f.source1.connect());
    f.source.add(f.source3.connect());

    let result = range(1, 5);
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &result);

    f.source2.add_range(range(6, 5));
    assert_eq!(f.results.data().count(), 5);

    f.source.add(f.source2.connect());
    assert_eq!(f.results.data().count(), 0);

    f.source.remove_at(2);
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &result);
}

#[test]
fn clear_one_clears_result() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add_range(range(1, 5));
    f.source2.add_range(range(1, 5));
    f.source1.clear();
    assert_eq!(f.results.data().count(), 0);
}

#[test]
fn combine_range() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add_range(range(1, 10));
    f.source2.add_range(range(6, 10));
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &range(6, 5));
}

#[test]
fn excluded_when_item_is_in_one_source() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add(1);
    assert_eq!(f.results.data().count(), 0);
}

#[test]
fn included_when_item_is_in_two_sources() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add(1);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 1);
}

#[test]
fn removed_when_no_longer_in_both() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add(1);
    f.source2.add(1);
    f.source1.remove(&1);
    assert_eq!(f.results.data().count(), 0);
}
