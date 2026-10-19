//! `ExceptFixture` (`DynamicData.Tests/Cache/ExceptFixture.cs`): the items of a cache that are not
//! in another, from a sequence with another (`ExceptFixture`) and from a collection of sequences
//! (`ExceptCollectionFixture`). The abstract `ExceptFixtureBase` is the functions of the tests,
//! which each fixture runs with its `CreateObservable`.

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{CacheSourcesCombineEx, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

type People = SourceCache<Rc<Person>, String>;

/// `CreateObservable`, of the target source and the source of the exceptions.
type CreateObservable = fn(&People, &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>;

/// `ExceptFixture.CreateObservable`: `_targetSource.Connect().Except(_exceptSource.Connect())`.
fn create_except(target_source: &People, except_source: &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>> {
    target_source.connect().except(vec![except_source.connect()])
}

/// `ExceptCollectionFixture.CreateObservable`: `new List { .. }.Except()`.
fn create_except_collection(target_source: &People, except_source: &People) -> Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>> {
    let l = vec![target_source.connect(), except_source.connect()];
    l.except()
}

/// `ExceptFixtureBase`: the two sources and the results.
struct Fixture {
    except_source: People,
    target_source: People,
    results: ChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new(create_observable: CreateObservable) -> Self {
        let target_source = SourceCache::new(|p: &Rc<Person>| p.name());
        let except_source = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = create_observable(&target_source, &except_source).as_aggregator();
        Self { except_source, target_source, results }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.target_source.dispose();
        self.except_source.dispose();
        self.results.dispose();
    }
}

fn do_not_include_except_list_items(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.except_source.add_or_update(person.clone());
    fixture.target_source.add_or_update(person);

    assert_eq!(fixture.results.messages().len(), 0, "Should have no updates");
    assert_eq!(fixture.results.data().count(), 0, "Cache should have no items");
}

fn removed_an_item_from_except_then_includes_the_item(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.except_source.add_or_update(person.clone());
    fixture.target_source.add_or_update(person.clone());

    fixture.except_source.remove(&person);
    assert_eq!(fixture.results.messages().len(), 1, "Should be 2 updates");
    assert_eq!(fixture.results.data().count(), 1, "Cache should have no items");
}

fn updating_one_source_only_produces_result(create_observable: CreateObservable) {
    let fixture = Fixture::new(create_observable);
    let person = Person::new("Adult1", 50);
    fixture.target_source.add_or_update(person);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
}

/// The `[Fact]`s of `ExceptFixtureBase`, for the fixture of `$create_observable`.
macro_rules! except_fixture_base_tests {
    ($create_observable:path) => {
        #[test]
        fn do_not_include_except_list_items() {
            super::do_not_include_except_list_items($create_observable);
        }

        #[test]
        fn removed_an_item_from_except_then_includes_the_item() {
            super::removed_an_item_from_except_then_includes_the_item($create_observable);
        }

        #[test]
        fn updating_one_source_only_produces_result() {
            super::updating_one_source_only_produces_result($create_observable);
        }
    };
}

/// `ExceptFixture`.
mod with_others {
    except_fixture_base_tests!(super::create_except);
}

/// `ExceptCollectionFixture`.
mod collection {
    except_fixture_base_tests!(super::create_except_collection);
}
