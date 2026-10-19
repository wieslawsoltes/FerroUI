//! `OrFixture` (`DynamicData.Tests/Cache/OrFixture.cs`): the items of two caches that are in
//! either, from a sequence with another (`OrFixture`) and from a collection of sequences
//! (`OrCollectionFixture`). The abstract `OrFixtureBase` is the functions of the tests, which each
//! fixture runs with its `CreateObservable`.

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{CacheSourcesCombineEx, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

type People = SourceCache<Rc<Person>, String>;

/// `CreateObservable`.
type CreateObservable = fn(&People, &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>;

/// `OrFixture.CreateObservable`: `_source1.Connect().Or(_source2.Connect())`.
fn create_or(source1: &People, source2: &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>> {
    source1.connect().or(vec![source2.connect()])
}

/// `OrCollectionFixture.CreateObservable`: `new List { .. }.Or()`.
fn create_or_collection(source1: &People, source2: &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>> {
    let l = vec![source1.connect(), source2.connect()];
    l.or()
}

/// `OrFixtureBase`: the two sources and the results.
struct Fixture {
    source1: People,
    source2: People,
    results: ChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new(create_observable: CreateObservable) -> Self {
        let source1 = SourceCache::new(|p: &Rc<Person>| p.name());
        let source2 = SourceCache::new(|p: &Rc<Person>| p.name());
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

fn removing_from_one_does_not_from_result(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.source1.add_or_update(person.clone());
    fixture.source2.add_or_update(person.clone());

    fixture.source2.remove(&person);
    assert_eq!(fixture.results.messages().len(), 1, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Cache should have no items");
}

fn updating_both_produces_results_and_does_not_duplicate_the_message(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.source1.add_or_update(person.clone());
    fixture.source2.add_or_update(person.clone());
    assert_eq!(fixture.results.messages().len(), 1, "Should have no updates");
    assert_eq!(fixture.results.data().count(), 1, "Cache should have no items");
    assert!(Rc::ptr_eq(&fixture.results.data().items()[0], &person), "Should be same person");
}

fn updating_one_produces_only_one_update(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.source1.add_or_update(person.clone());
    fixture.source2.add_or_update(person);

    let person_updated = Person::new("Adult1", 51);
    fixture.source2.add_or_update(person_updated.clone());
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Cache should have no items");
    assert!(Rc::ptr_eq(&fixture.results.data().items()[0], &person_updated), "Should be updated person");
}

fn updating_one_source_only_produces_result(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.source1.add_or_update(person);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
}

/// The `[Fact]`s of `OrFixtureBase`, for the fixture of `$create_observable`.
macro_rules! or_fixture_base_tests {
    ($create_observable:path) => {
        #[test]
        fn removing_from_one_does_not_from_result() {
            super::removing_from_one_does_not_from_result($create_observable);
        }

        #[test]
        fn updating_both_produces_results_and_does_not_duplicate_the_message() {
            super::updating_both_produces_results_and_does_not_duplicate_the_message($create_observable);
        }

        #[test]
        fn updating_one_produces_only_one_update() {
            super::updating_one_produces_only_one_update($create_observable);
        }

        #[test]
        fn updating_one_source_only_produces_result() {
            super::updating_one_source_only_produces_result($create_observable);
        }
    };
}

/// `OrFixture`.
mod with_others {
    or_fixture_base_tests!(super::create_or);
}

/// `OrCollectionFixture`.
mod collection {
    or_fixture_base_tests!(super::create_or_collection);
}
