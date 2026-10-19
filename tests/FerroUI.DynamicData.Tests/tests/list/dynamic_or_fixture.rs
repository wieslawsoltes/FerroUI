//! `DynamicOrFixture` (`DynamicData.Tests/List/DynamicOrFixture.cs`): the items of the lists of a
//! list that are in any of them, as lists are added to the list and removed from it
//! (`Or(IObservableList<IObservable<IChangeSet<T>>>)`), with the refreshes they pass through
//! (`DynamicOrRefreshFixture`).
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
        let results = source.or().as_aggregator();
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
    f.source2.add_range(range(6, 5));
    f.source3.add_range(range(100, 5));

    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());
    f.source.add(f.source3.connect());

    let result: Vec<i32> = range(1, 5).into_iter().chain(range(6, 5)).chain(range(100, 5)).collect();
    assert_eq!(f.results.data().count(), 15);
    assert_equivalent(&f.results.data().items(), &result);

    f.source.remove_at(1);
    assert_eq!(f.results.data().count(), 10);
    let result: Vec<i32> = range(1, 5).into_iter().chain(range(100, 5)).collect();
    assert_equivalent(&f.results.data().items(), &result);
}

#[test]
fn clear_only_clears_one_source() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add_range(range(1, 5));
    f.source2.add_range(range(6, 5));
    f.source1.clear();
    assert_eq!(f.results.data().count(), 5);
    assert_equivalent(&f.results.data().items(), &range(6, 5));
}

#[test]
fn clear_source() {
    let f = Fixture::new();
    f.source1.add(0);
    f.source2.add(1);
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());
    f.source.clear();
    assert_eq!(f.results.data().count(), 0);
}

#[test]
fn combine_range() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add_range(range(1, 5));
    f.source2.add_range(range(6, 5));
    assert_eq!(f.results.data().count(), 10);
    assert_equivalent(&f.results.data().items(), &range(1, 10));
}

#[test]
fn included_when_item_is_in_one_source() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add(1);
    assert_eq!(f.results.data().count(), 1);
    assert_eq!(f.results.data().items()[0], 1);
}

#[test]
fn included_when_item_is_in_two_sources() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add(1);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 1);
    assert_eq!(f.results.data().items()[0], 1);
}

#[test]
fn item_is_replaced() {
    let f = Fixture::new();
    f.source1.add(0);
    f.source2.add(1);
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());
    f.source1.replace_at(0, 9);

    assert_eq!(f.results.data().count(), 2);
    assert_eq!(f.results.messages().len(), 3);
    assert_equivalent(&f.results.data().items(), &[9, 1]);
}

#[test]
fn remove_all_lists() {
    let f = Fixture::new();
    f.source1.add_range(range(1, 5));
    f.source3.add_range(range(100, 5));

    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());
    f.source.add(f.source3.connect());

    f.source2.add_range(range(6, 5));
    f.source.clear();
    assert_eq!(f.results.data().count(), 0);
}

#[test]
fn removed_when_no_longer_in_either() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    f.source1.add(1);
    f.source1.remove(&1);
    assert_eq!(f.results.data().count(), 0);
}

/// `DynamicOrRefreshFixture`.
mod dynamic_or_refresh_fixture {
    use super::super::or_fixture::Item;
    use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ListSourceListCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
    use ferroui_system::IObservable;
    use std::rc::Rc;

    #[test]
    fn refresh_passes_through() {
        let source1 = SourceList::<Rc<Item>>::new();
        let source2 = SourceList::<Rc<Item>>::new();
        let source = SourceList::<Rc<dyn IObservable<ChangeSet<Rc<Item>>>>>::new();
        let results = source.or().as_aggregator();
        source1.add(Item::new("A"));
        source2.add(Item::new("B"));
        source.add_range(vec![source1.connect().auto_refresh_any(), source2.connect().auto_refresh_any()]);

        source1.items()[0].set_name("Test");

        assert_eq!(results.data().count(), 2);
        assert_eq!(results.messages().len(), 3);
        assert_eq!(results.messages()[2].refreshes(), 1);
        assert!(Rc::ptr_eq(results.messages()[2].changes()[0].item().current(), &source1.items()[0]));
    }
}
