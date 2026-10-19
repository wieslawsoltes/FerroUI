//! `ObservableCollectionExtendedToChangeSetFixture`
//! (`DynamicData.Tests/Binding/ObservableCollectionExtendedToChangeSetFixture.cs`).

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{ObservableCollectionEx, ObservableCollectionExtended, ReadOnlyObservableCollection};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};

struct Fixture {
    collection: ObservableCollectionExtended<i32>,
    results: ChangeSetAggregator<i32>,
    target: ReadOnlyObservableCollection<i32>,
}

impl Fixture {
    fn new() -> Self {
        let collection = ObservableCollectionExtended::new();
        let target = ReadOnlyObservableCollection::from(&collection);
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
    fixture.collection.add_range(1..=10);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());

    fixture.collection.move_item(5, 8);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());

    fixture.collection.move_item(7, 1);
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    fixture.collection.add_range(1..=10);

    fixture.collection.remove(&3);

    assert_eq!(fixture.results.data().count(), 9);
    assert!(!fixture.results.data().items().contains(&3));
    assert_equivalent(&fixture.results.data().items(), &fixture.target.to_vec());
}

#[test]
fn replace() {
    let fixture = Fixture::new();
    fixture.collection.add_range(1..=10);
    fixture.collection.set(8, 20);

    assert_equivalent(&fixture.results.data().items(), &[1, 2, 3, 4, 5, 6, 7, 8, 20, 10]);
}

// Not ported: ResetFiresClearsAndAdds - commented out upstream.
