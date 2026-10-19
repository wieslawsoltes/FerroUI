//! `TransformManyFixture` (`DynamicData.Tests/Cache/TransformManyFixture.cs`): the relations of
//! people, recursively, as one cache.

use crate::domain::PersonWithRelations;
use crate::utilities::SelectManyExtensions;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Rc<PersonWithRelations>, String>,
    source: SourceCache<Rc<PersonWithRelations>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<PersonWithRelations>| p.key());

        let results = source
            .connect()
            .transform_many(|p: &Rc<PersonWithRelations>| p.relations().recursive_select(|r| r.relations()), |p: &Rc<PersonWithRelations>| p.name())
            .ignore_update_when(|current: &Rc<PersonWithRelations>, previous: &Rc<PersonWithRelations>| current.name() == previous.name())
            .as_aggregator();
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
fn children_are_removed_when_parent_is_removed() {
    let fixture = Fixture::new();
    let frientofchild1 = PersonWithRelations::new("Friend1", 10);
    let child1 = PersonWithRelations::with_relations("Child1", 10, vec![frientofchild1]);
    let child2 = PersonWithRelations::new("Child2", 8);
    let child3 = PersonWithRelations::new("Child3", 8);
    let mother = PersonWithRelations::with_relations("Mother", 35, vec![child1, child2, child3]);
    //  var father = new PersonWithRelations("Father", 35, new[] {child1, child2, child3, mother});

    fixture.source.edit(|updater| updater.add_or_update(mother.clone()));
    fixture.source.edit(|updater| updater.remove_item(&mother));
    assert_eq!(fixture.results.data().count(), 0, "Should be 4 in the cache");
}

#[test]
fn recursive_children_can_be_added() {
    let fixture = Fixture::new();
    let frientofchild1 = PersonWithRelations::new("Friend1", 10);
    let child1 = PersonWithRelations::with_relations("Child1", 10, vec![frientofchild1]);
    let child2 = PersonWithRelations::new("Child2", 8);
    let child3 = PersonWithRelations::new("Child3", 8);
    let mother = PersonWithRelations::with_relations("Mother", 35, vec![child1, child2, child3]);
    //  var father = new PersonWithRelations("Father", 35, new[] {child1, child2, child3, mother});

    fixture.source.edit(|updater| updater.add_or_update(mother));

    assert_eq!(fixture.results.data().count(), 4, "Should be 4 in the cache");
    assert!(fixture.results.data().lookup(&"Child1".to_owned()).is_some());
    assert!(fixture.results.data().lookup(&"Child2".to_owned()).is_some());
    assert!(fixture.results.data().lookup(&"Child3".to_owned()).is_some());
    assert!(fixture.results.data().lookup(&"Friend1".to_owned()).is_some());
}
