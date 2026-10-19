//! `ReadOnlyObservableCollectionToChangeSetFixture`
//! (`DynamicData.Tests/Binding/ReadOnlyObservableCollectionToChangeSetFixture.cs`).

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{AbstractNotifyPropertyChanged, ObservableCollectionEx, ReadOnlyObservableCollection};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_dynamic_data::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// `TestObservableCollection<T>`: an observable collection that raises a reset on demand.
fn reset<T: Clone + PartialEq + 'static>(collection: &ObservableCollection<T>) {
    collection.on_collection_reset();
}

fn add_range(collection: &ObservableCollection<i32>, items: impl IntoIterator<Item = i32>) {
    for item in items {
        collection.add(item);
    }
}

struct Fixture {
    collection: ObservableCollection<i32>,
    results: ChangeSetAggregator<i32>,
    target: ReadOnlyObservableCollection<i32>,
}

impl Fixture {
    fn new() -> Self {
        let collection = ObservableCollection::new();
        let target = ReadOnlyObservableCollection::new(collection.clone());
        let results = target.to_observable_change_set().as_aggregator();
        Self { collection, results, target }
    }
}

#[test]
fn add() {
    let fixture = Fixture::new();
    fixture.collection.add(1);

    assert_eq!(fixture.results.messages().len(), 2);
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(fixture.results.data().items()[0], 1);
}

#[test]
fn duplicates() {
    let fixture = Fixture::new();
    fixture.collection.add(1);
    fixture.collection.add(1);

    assert_eq!(fixture.results.data().count(), 2);
}

#[test]
fn move_() {
    let fixture = Fixture::new();
    add_range(&fixture.collection, 1..=10);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());

    fixture.collection.move_item(5, 8);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());

    fixture.collection.move_item(7, 1);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());
}

/// `Item`: an object with an id and a name that notifies its changes (the class of the test).
struct Item {
    id: u64,
    name: RefCell<String>,
    notify: AbstractNotifyPropertyChanged,
}

impl Item {
    fn new(name: &str) -> Rc<Self> {
        // The `Guid` of upstream: an id of its own.
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Rc::new(Self { id, name: RefCell::new(name.to_owned()), notify: AbstractNotifyPropertyChanged::new() })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: &str) {
        *self.name.borrow_mut() = value.to_owned();
        self.notify.on_property_changed("Name");
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl INotifyPropertyChanged for Item {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

#[test]
fn refresh_not_supported() {
    // Arrange
    let source_cache = SourceCache::new(|item: &Rc<Item>| item.id);
    let item1 = Item::new("Old Name");
    source_cache.edit(|updater| updater.add_or_update(item1.clone()));
    let (bound, collection) = source_cache.connect().auto_refresh(ferroui_system::prop!(Item => name)).bind_read_only();
    let source_cache_results = bound.as_aggregator();
    let collection_results = collection.to_observable_change_set().as_aggregator();

    item1.set_name("New Name");

    // The source cache received an add and a refresh.
    assert_eq!(source_cache_results.messages().len(), 2);
    assert_eq!(source_cache_results.messages()[0].adds(), 1);
    assert_eq!(source_cache_results.messages().last().expect("a refresh").refreshes(), 1);

    // The collection only receives the add and NOT the refresh:
    // NotifyCollectionChangedAction has no action that describes a refresh of the same item.
    assert_eq!(collection_results.messages().len(), 1);
    assert_eq!(collection_results.messages()[0].adds(), 1);

    source_cache.dispose();
    source_cache_results.dispose();
    collection_results.dispose();
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    add_range(&fixture.collection, 1..=10);

    fixture.collection.remove(&3);

    assert_eq!(fixture.results.data().count(), 9);
    assert!(!fixture.results.data().items().contains(&3));
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());
}

#[test]
fn replace() {
    let fixture = Fixture::new();
    add_range(&fixture.collection, 1..=10);
    fixture.collection.set(8, 20);

    assert_equivalent(&fixture.results.data().items(), &[1, 2, 3, 4, 5, 6, 7, 8, 20, 10]);
}

#[test]
fn reset_fires_clears_and_adds() {
    let fixture = Fixture::new();
    add_range(&fixture.collection, 1..=10);

    reset(&fixture.collection);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());

    let reset_notification = fixture.results.messages().last().cloned().expect("a reset");
    assert_eq!(reset_notification.removes(), 10);
    assert_eq!(reset_notification.adds(), 10);
}
