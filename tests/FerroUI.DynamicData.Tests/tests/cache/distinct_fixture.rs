//! `DistinctFixture` (`DynamicData.Tests/Cache/DistinctFixture.cs`): the distinct values of the
//! items of a cache (`DistinctValues`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `DistinctChangeSetAggregator<int>`.
type DistinctAggregator = ChangeSetAggregator<i32, i32>;

/// The fixture (`DistinctFixture` with its constructor and `Dispose`).
struct Fixture {
    results: DistinctAggregator,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results: DistinctAggregator = source.connect().distinct_values(|p: &Rc<Person>| p.age()).as_aggregator();
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn break_with_loads_of_updates() {
    let fixture = Fixture::new();

    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new("Person2", 12));
        updater.add_or_update(Person::new("Person1", 1));
        updater.add_or_update(Person::new("Person1", 1));
        updater.add_or_update(Person::new("Person2", 12));

        updater.add_or_update(Person::new("Person3", 13));
        updater.add_or_update(Person::new("Person4", 14));
    });

    assert_equivalent(&fixture.results.data().items(), &[1, 12, 13, 14]);

    // This previously threw
    fixture.source.remove(&Person::new("Person3", 13));

    assert_equivalent(&fixture.results.data().items(), &[1, 12, 14]);
}

#[test]
fn duplicated_results_result_in_no_additional_message() {
    let fixture = Fixture::new();

    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person1", 20));
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 update message");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 items in the cache");
    assert_eq!(fixture.results.data().items()[0], 20, "Should 20");
}

#[test]
fn duplicate_keys_refresh_after_remove() {
    let _fixture = Fixture::new();

    let source1 = SourceCache::new(|p: &Rc<Person>| p.name());
    let source2 = SourceCache::new(|p: &Rc<Person>| p.name());

    let person = Person::new("Person2", 12);

    let results: DistinctAggregator = source1.connect().merge(source2.connect()).distinct_values(|p: &Rc<Person>| p.age()).as_aggregator();

    source1.add_or_update(person.clone());
    source2.add_or_update(person.clone());
    source2.remove(&person);
    source1.refresh(&person); // would previously throw KeyNotFoundException here

    assert_eq!(results.messages().len(), 1);
    assert_equivalent(&results.data().items(), &[12]);

    source1.remove(&person);

    assert_eq!(results.messages().len(), 2);
    assert!(results.data().items().is_empty());
}

#[test]
fn fires_add_whena_new_item_is_added() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("Person1", 20));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
    assert_eq!(fixture.results.data().items()[0], 20, "Should 20");
}

#[test]
fn fires_batch_result_once() {
    let fixture = Fixture::new();

    fixture.source.edit(|updater| {
        updater.add_or_update(Person::new("Person1", 20));
        updater.add_or_update(Person::new("Person2", 21));
        updater.add_or_update(Person::new("Person3", 22));
    });

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 3, "Should be 3 items in the cache");

    assert_equivalent(&fixture.results.data().items(), &[20, 21, 22]);
    assert_eq!(fixture.results.data().items()[0], 20, "Should 20");
}

#[test]
fn removing_an_item_removes_the_distinct() {
    let fixture = Fixture::new();

    fixture.source.add_or_update(Person::new("Person1", 20));
    fixture.source.remove(&Person::new("Person1", 20));
    let messages = fixture.results.messages();
    assert_eq!(messages.len(), 2, "Should be 1 update message");
    assert_eq!(fixture.results.data().count(), 0, "Should be 1 items in the cache");

    assert_eq!(messages[0].adds(), 1, "First message should be an add");
    assert_eq!(messages[1].removes(), 1, "Second messsage should be a remove");
}
