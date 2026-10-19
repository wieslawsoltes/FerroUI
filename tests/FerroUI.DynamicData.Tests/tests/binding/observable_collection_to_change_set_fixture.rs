//! `ObservableCollectionToChangeSetFixture`
//! (`DynamicData.Tests/Binding/ObservableCollectionToChangeSetFixture.cs`).

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::ObservableCollectionEx;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_system::collections::object_model::ObservableCollection;

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
}

impl Fixture {
    fn new() -> Self {
        let collection = ObservableCollection::new();
        let results = collection.to_observable_change_set().as_aggregator();
        Self { collection, results }
    }
}

#[test]
fn add() {
    let fixture = Fixture::new();
    fixture.collection.add(1);

    assert!(fixture.results.messages().len() >= 1);
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
    assert_equivalent(&fixture.results.data().items(), &fixture.collection.to_vec());

    fixture.collection.move_item(5, 8);
    assert_equivalent(&fixture.results.data().items(), &fixture.collection.to_vec());

    fixture.collection.move_item(7, 1);
    assert_equivalent(&fixture.results.data().items(), &fixture.collection.to_vec());
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    add_range(&fixture.collection, 1..=10);

    fixture.collection.remove(&3);

    assert_eq!(fixture.results.data().count(), 9);
    assert!(!fixture.results.data().items().contains(&3));
    assert_equivalent(&fixture.results.data().items(), &fixture.collection.to_vec());
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
    assert_equivalent(&fixture.results.data().items(), &fixture.collection.to_vec());

    let reset_notification = fixture.results.messages().last().cloned().expect("a reset");
    assert_eq!(reset_notification.removes(), 10);
    assert_eq!(reset_notification.adds(), 10);
}
