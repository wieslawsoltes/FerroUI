//! `DynamicAndFixture` (`DynamicData.Tests/Cache/DynamicAndFixture.cs`): the items of the caches of
//! a list that are in all of them, as caches are added to the list and removed from it
//! (`And(IObservableList<IObservable<IChangeSet>>)`).
//!
//! The list holds sequences, which compare by identity (`SourceList<IObservable<..>>`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{CacheSourceListCombineEx, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_dynamic_data::list::{SourceList, SourceListEditConvenienceEx};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// The change sets of a cache of people (`IObservable<IChangeSet<Person, string>>`).
type PeopleChanges = Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>;

/// The fixture: the list of sequences, the three caches and the results.
struct Fixture {
    generator: RandomPersonGenerator,
    results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceList<PeopleChanges>,
    source1: SourceCache<Rc<Person>, String>,
    source2: SourceCache<Rc<Person>, String>,
    source3: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source1 = SourceCache::new(|p: &Rc<Person>| p.name());
        let source2 = SourceCache::new(|p: &Rc<Person>| p.name());
        let source3 = SourceCache::new(|p: &Rc<Person>| p.name());
        let source: SourceList<PeopleChanges> = SourceList::new();
        let results = source.and_cache().as_aggregator();
        Self { generator: RandomPersonGenerator::new(), results, source, source1, source2, source3 }
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

#[test]
fn add_and_remove_lists() {
    let f = Fixture::new();
    let items = f.generator.take(100);
    f.source1.add_or_update_items(items[..20].iter().cloned());
    f.source2.add_or_update_items(items[10..20].iter().cloned());

    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    assert_eq!(f.results.data().count(), 10);
    assert_equivalent(&f.results.data().items(), &items[10..20]);

    f.source.add(f.source3.connect());
    assert_eq!(f.results.data().count(), 0);

    f.source.remove_at(2);
    assert_eq!(f.results.data().count(), 10);
}

#[test]
fn remove_all_lists() {
    let f = Fixture::new();
    let items = f.generator.take(100);

    f.source1.add_or_update_items(items[..10].iter().cloned());
    f.source2.add_or_update_items(items[20..30].iter().cloned());
    f.source3.add_or_update_items(items[30..].iter().cloned());

    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());
    f.source.add(f.source3.connect());

    f.source.remove_at(2);
    f.source.remove_at(1);
    f.source.remove_at(0);
    //s _source.Clear();

    assert_eq!(f.results.data().count(), 0);
}

#[test]
fn removing_from_one_removes_from_result() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    let person = Person::new("Adult1", 50);
    f.source1.add_or_update(person.clone());
    f.source2.add_or_update(person.clone());

    f.source2.remove(&person);
    assert_eq!(f.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(f.results.data().count(), 0, "Cache should have no items");
}

#[test]
fn updating_both_produces_results() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    let person = Person::new("Adult1", 50);
    f.source1.add_or_update(person.clone());
    f.source2.add_or_update(person.clone());
    assert_eq!(f.results.messages().len(), 1, "Should have no updates");
    assert_eq!(f.results.data().count(), 1, "Cache should have no items");
    assert!(Rc::ptr_eq(&f.results.data().items()[0], &person), "Should be same person");
}

#[test]
fn updating_one_produces_only_one_update() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    let person = Person::new("Adult1", 50);
    f.source1.add_or_update(person.clone());
    f.source2.add_or_update(person);

    let person_updated = Person::new("Adult1", 51);
    f.source2.add_or_update(person_updated.clone());
    assert_eq!(f.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(f.results.data().count(), 1, "Cache should have no items");
    assert!(Rc::ptr_eq(&f.results.data().items()[0], &person_updated), "Should be updated person");
}

#[test]
fn updating_one_source_only_produces_no_results() {
    let f = Fixture::new();
    f.source.add(f.source1.connect());
    f.source.add(f.source2.connect());

    let person = Person::new("Adult1", 50);
    f.source1.add_or_update(person);

    assert_eq!(f.results.messages().len(), 0, "Should have no updates");
    assert_eq!(f.results.data().count(), 0, "Cache should have no items");
}
