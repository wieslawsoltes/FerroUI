//! `TransformManySimpleFixture` (`DynamicData.Tests/Cache/TransformManySimpleFixture.cs`): the
//! relations of people as one cache, as the people are added, updated and removed.

use crate::domain::{Person, PersonWithChildren};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<PersonWithChildren>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<PersonWithChildren>| p.key());

        let results = source.connect().transform_many(|p: &Rc<PersonWithChildren>| p.relations(), |p: &Rc<Person>| p.name()).as_aggregator();
        Self { results, source }
    }

    fn add_or_update(&self, parent: &Rc<PersonWithChildren>) {
        self.source.edit(|updater| updater.add_or_update(parent.clone()));
    }

    fn remove(&self, parent: &Rc<PersonWithChildren>) {
        self.source.edit(|updater| updater.remove_item(parent));
    }

    fn contains(&self, key: &str) -> bool {
        self.results.data().lookup(&key.to_owned()).is_some()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn adds() {
    let fixture = Fixture::new();
    let parent = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child2", 2), Person::new("Child3", 3)]);
    fixture.add_or_update(&parent);
    assert_eq!(fixture.results.data().count(), 3, "Should be 4 in the cache");

    assert!(fixture.contains("Child1"));
    assert!(fixture.contains("Child2"));
    assert!(fixture.contains("Child3"));
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    let parent = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child2", 2), Person::new("Child3", 3)]);
    fixture.add_or_update(&parent);
    fixture.remove(&parent);
    assert_eq!(fixture.results.data().count(), 0, "Should be 4 in the cache");
}

#[test]
fn removewith_incomplete_children() {
    let fixture = Fixture::new();
    let parent1 = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child2", 2), Person::new("Child3", 3)]);
    fixture.add_or_update(&parent1);

    let parent2 = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child3", 3)]);
    fixture.remove(&parent2);
    assert_eq!(fixture.results.data().count(), 0, "Should be 0 in the cache");
}

#[test]
fn update_with_less_children() {
    let fixture = Fixture::new();
    let parent1 = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child2", 2), Person::new("Child3", 3)]);
    fixture.add_or_update(&parent1);

    let parent2 = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child3", 3)]);
    fixture.add_or_update(&parent2);
    assert_eq!(fixture.results.data().count(), 2, "Should be 2 in the cache");
    assert!(fixture.contains("Child1"));
    assert!(fixture.contains("Child3"));
}

#[test]
fn update_with_multiple_changes() {
    let fixture = Fixture::new();
    let parent1 = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child2", 2), Person::new("Child3", 3)]);
    fixture.add_or_update(&parent1);

    let parent2 = PersonWithChildren::with_relations("parent", 50, vec![Person::new("Child1", 1), Person::new("Child3", 3), Person::new("Child5", 3)]);
    fixture.add_or_update(&parent2);
    assert_eq!(fixture.results.data().count(), 3, "Should be 2 in the cache");
    assert!(fixture.contains("Child1"));
    assert!(fixture.contains("Child3"));
    assert!(fixture.contains("Child5"));
}
