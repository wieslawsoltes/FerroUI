//! `AndFixture` (`DynamicData.Tests/Cache/AndFixture.cs`): the items of two caches that are in
//! both, from a sequence with another (`AndFixture`) and from a collection of sequences
//! (`AndCollectionFixture`). The abstract `AndFixtureBase` is the functions of the tests, which
//! each fixture runs with its `CreateObservable`.

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{CacheSourcesCombineEx, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

type People = SourceCache<Rc<Person>, String>;

/// `CreateObservable`.
type CreateObservable = fn(&People, &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>;

/// `AndFixture.CreateObservable`: `_source1.Connect().And(_source2.Connect())`.
fn create_and(source1: &People, source2: &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>> {
    source1.connect().and_cache(vec![source2.connect()])
}

/// `AndCollectionFixture.CreateObservable`: `new List { .. }.And()`.
fn create_and_collection(source1: &People, source2: &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>> {
    let l = vec![source1.connect(), source2.connect()];
    l.and_cache()
}

/// `AndFixtureBase`: the two sources and the results.
struct Fixture {
    source1: People,
    source2: People,
    results: ChangeSetAggregator<Rc<Person>, String>,
    create_observable: CreateObservable,
}

impl Fixture {
    fn new(create_observable: CreateObservable) -> Self {
        let source1 = SourceCache::new(|p: &Rc<Person>| p.name());
        let source2 = SourceCache::new(|p: &Rc<Person>| p.name());
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

fn removing_from_one_removes_from_result(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.source1.add_or_update(person.clone());
    fixture.source2.add_or_update(person.clone());

    fixture.source2.remove(&person);
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 0, "Cache should have no items");
}

fn starting_with_non_empty_source_produces_no_result(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult", 50);
    fixture.source1.add_or_update(person);

    let result = (fixture.create_observable)(&fixture.source1, &fixture.source2).as_aggregator();
    assert_eq!(fixture.results.messages().len(), 0, "Should have no updates");
    assert_eq!(result.data().count(), 0, "Cache should have no items");
    result.dispose();
}

fn updating_both_produces_results(create_observable: CreateObservable) {
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

fn updating_one_source_only_produces_no_results(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.source1.add_or_update(person);

    assert_eq!(fixture.results.messages().len(), 0, "Should have no updates");
    assert_eq!(fixture.results.data().count(), 0, "Cache should have no items");
}

/// The `[Fact]`s of `AndFixtureBase`, for the fixture of `$create_observable`.
macro_rules! and_fixture_base_tests {
    ($create_observable:path) => {
        #[test]
        fn removing_from_one_removes_from_result() {
            super::removing_from_one_removes_from_result($create_observable);
        }

        #[test]
        fn starting_with_non_empty_source_produces_no_result() {
            super::starting_with_non_empty_source_produces_no_result($create_observable);
        }

        #[test]
        fn updating_both_produces_results() {
            super::updating_both_produces_results($create_observable);
        }

        #[test]
        fn updating_one_produces_only_one_update() {
            super::updating_one_produces_only_one_update($create_observable);
        }

        #[test]
        fn updating_one_source_only_produces_no_results() {
            super::updating_one_source_only_produces_no_results($create_observable);
        }
    };
}

/// `AndFixture`.
mod with_others {
    and_fixture_base_tests!(super::create_and);
}

/// `AndCollectionFixture`.
mod collection {
    and_fixture_base_tests!(super::create_and_collection);
}
