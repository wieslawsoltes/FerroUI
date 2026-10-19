//! `TransformManyObservableCollectionFixture` (`DynamicData.Tests/Cache/TransformManyObservableCacheFixture.cs`):
//! the children of parents as one cache, followed as the collections of children change.

use crate::domain::Person;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

// Not ported: FlattenObservableCollection - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: FlattenReadOnlyObservableCollection - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: ObservableCollectionWithoutInitialData - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: ReadOnlyObservableCollectionWithoutInitialData - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)

/// `Parent`, with the form of `ChildrenCache` only.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream keeps the children in an
/// `ObservableCollection<Person>` and makes `ChildrenCache` of it with
/// `ToObservableChangeSet(x => x.Name).AsObservableCache()`, which waits for wave 2; the children
/// are a `SourceCache` here, whose observable cache is `ChildrenCache`. A replacement in the
/// collection, which the cache sees as the removal of the old child and the addition of the new,
/// is that edit of the source.
struct Parent {
    id: i32,
    children: SourceCache<Rc<Person>, String>,
    children_cache: ObservableCache<Rc<Person>, String>,
}

impl Parent {
    fn new(id: i32, children: Vec<Rc<Person>>) -> Rc<Self> {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        source.edit(|updater| updater.add_or_update_items(&mut children.into_iter()));
        let children_cache = source.connect().as_observable_cache();
        Rc::new(Self { id, children: source, children_cache })
    }

    fn add_child(&self, child: Rc<Person>) {
        self.children.edit(|updater| updater.add_or_update(child));
    }

    fn child_items(&self) -> Vec<Rc<Person>> {
        self.children.items()
    }
}

fn make_parents() -> Vec<Rc<Parent>> {
    let children: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();
    (1..=50).map(|i| {
        let child_index = (i as usize - 1) * 2;
        Parent::new(i, vec![children[child_index].clone(), children[child_index + 1].clone()])
    }).collect()
}

#[test]
fn flatten_observable_cache() {
    let parents = make_parents();

    let source = SourceCache::new(|x: &Rc<Parent>| x.id);
    let aggregator = source.connect().transform_many_observable_cache(|p: &Rc<Parent>| p.children_cache.clone(), |c: &Rc<Person>| c.name()).as_aggregator();
    source.edit(|updater| updater.add_or_update_items(&mut parents.iter().cloned()));

    assert_eq!(aggregator.data().count(), 100);

    // Add a child to an observable collection and check the new item is added.
    parents[0].add_child(Person::new("NewlyAddded", 100));
    assert_eq!(aggregator.data().count(), 101);

    // Remove the first parent and check its children have gone.
    source.edit(|updater| updater.remove_key(&1));
    assert_eq!(aggregator.data().count(), 98);

    // Check items can be cleared and then added back in.
    let children_in_zero = parents[1].child_items();
    parents[1].children.edit(|updater| updater.clear());
    assert_eq!(aggregator.data().count(), 96);
    parents[1].children.edit(|updater| updater.add_or_update_items(&mut children_in_zero.iter().cloned()));
    assert_eq!(aggregator.data().count(), 98);

    // Replace produces an update.
    let replaced_child = parents[1].child_items()[0].clone();
    parents[1].children.edit(|updater| {
        updater.remove_key(&replaced_child.name());
        updater.add_or_update(Person::new("Replacement", 100));
    });
    assert_eq!(aggregator.data().count(), 98);

    assert!(aggregator.data().lookup(&replaced_child.key()).is_none());
    assert!(aggregator.data().lookup(&"Replacement".to_owned()).is_some());

    source.dispose();
}

#[test]
fn observable_cache_without_initial_data() {
    let parents = SourceCache::new(|d: &Rc<Parent>| d.id);
    let collection = parents.connect().transform_many_observable_cache(|d: &Rc<Parent>| d.children_cache.clone(), |p: &Rc<Person>| p.name()).as_observable_cache();

    let parent = Parent::new(1, Vec::new());
    parents.edit(|updater| updater.add_or_update(parent.clone()));

    assert_eq!(collection.count(), 0);

    parent.add_child(Person::new("child1", 1));
    assert_eq!(collection.count(), 1);

    parent.add_child(Person::new("child2", 2));
    assert_eq!(collection.count(), 2);

    parents.dispose();
}
