//! `OrFixture` (`DynamicData.Tests/List/OrFixture.cs`): the items of two lists that are in
//! either, from a sequence with another (`OrFixture`) and from a collection of sequences
//! (`OrCollectionFixture`), with the refreshes (`OrRefreshFixture`) and the replacements
//! (`OrReplaceFixture`) they pass through. The abstract `OrFixtureBase` is the functions of the
//! tests, which each fixture runs with its `CreateObservable`.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ListSourcesCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable};
use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// `CreateObservable`.
type CreateObservable = fn(&SourceList<i32>, &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>>;

/// `OrFixture.CreateObservable`: `_source1.Connect().Or(_source2.Connect())`.
fn create_with_others(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    source1.connect().or(vec![source2.connect()])
}

/// `OrCollectionFixture.CreateObservable`: `new List { .. }.Or()`.
fn create_collection(source1: &SourceList<i32>, source2: &SourceList<i32>) -> Rc<dyn IObservable<ChangeSet<i32>>> {
    let l = vec![source1.connect(), source2.connect()];
    l.or()
}

/// `OrFixtureBase`: the two sources and the results.
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

fn included_when_item_is_in_two_sources(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source2.add(1);
    assert_eq!(f.results.data().count(), 1);
    assert_eq!(f.results.data().items()[0], 1);
}

fn removed_when_no_longer_in_either(create_observable: CreateObservable) {
    let f = Fixture::new(create_observable);
    f.source1.add(1);
    f.source1.remove(&1);
    assert_eq!(f.results.data().count(), 0);
}
/// The `[Fact]`s of `OrFixtureBase`, for the fixture of `$create_observable`.
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
        fn included_when_item_is_in_two_sources() {
            super::included_when_item_is_in_two_sources($create_observable);
        }

        #[test]
        fn removed_when_no_longer_in_either() {
            super::removed_when_no_longer_in_either($create_observable);
        }
    };
}

/// `OrFixture`.
mod with_others {
    fixture_base_tests!(super::create_with_others);
}

/// `OrCollectionFixture`.
mod collection {
    fixture_base_tests!(super::create_collection);
}

/// An item with a name that raises `PropertyChanged` for `Name` on every assignment (`Item` of
/// `AutoRefreshFilter.cs`). Items are held in `Rc` and are equal only to themselves, as the
/// reference type of upstream; they hash by their address. `DynamicOrFixture` uses it too.
pub(crate) struct Item {
    notify: AbstractNotifyPropertyChanged,
    name: RefCell<String>,
}

impl Item {
    /// `new Item(name)`.
    pub(crate) fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), name: RefCell::new(name.to_owned()) })
    }

    /// Sets `Name` and raises `Name`.
    pub(crate) fn set_name(&self, value: &str) {
        *self.name.borrow_mut() = value.to_owned();
        self.notify.on_property_changed("Name");
    }
}

impl INotifyPropertyChanged for Item {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Item {}

impl Hash for Item {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item").field("name", &*self.name.borrow()).finish()
    }
}

/// `OrRefreshFixture`.
mod or_refresh_fixture {
    use super::Item;
    use ferroui_dynamic_data::list::{IObservableList, ListSourcesCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
    use std::rc::Rc;

    #[test]
    fn refresh_passes_through() {
        let source1 = SourceList::<Rc<Item>>::new();
        source1.add(Item::new("A"));
        let source2 = SourceList::<Rc<Item>>::new();
        source2.add(Item::new("B"));
        let list = vec![source1.connect().auto_refresh_any(), source2.connect().auto_refresh_any()];
        let results = list.or().as_aggregator();

        source1.items()[0].set_name("Test");

        assert_eq!(results.data().count(), 2);
        assert_eq!(results.messages().len(), 3);
        assert_eq!(results.messages()[2].refreshes(), 1);
        assert!(Rc::ptr_eq(results.messages()[2].changes()[0].item().current(), &source1.items()[0]));
    }
}

/// `OrReplaceFixture`.
mod or_replace_fixture {
    use super::Item;
    use crate::utilities::assert_equivalent;
    use ferroui_dynamic_data::list::{IObservableList, ListSourcesCombineEx, ObservableListEx, SourceList, SourceListEditConvenienceEx};
    use std::rc::Rc;

    #[test]
    fn item_is_replaced() {
        let item1 = Item::new("A");
        let item2 = Item::new("B");
        let item1_replacement = Item::new("Test");
        let source1 = SourceList::<Rc<Item>>::new();
        source1.add(item1);
        let source2 = SourceList::<Rc<Item>>::new();
        source2.add(item2.clone());
        let list = vec![source1.connect(), source2.connect()];
        let results = list.or().as_aggregator();

        source1.replace_at(0, item1_replacement.clone());

        assert_eq!(results.data().count(), 2);
        assert_eq!(results.messages().len(), 3);
        assert_equivalent(&results.data().items(), &[item1_replacement, item2]);
    }
}
