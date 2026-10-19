//! `TransformManyObservableCollectionFixture` (`DynamicData.Tests/Cache/TransformManyObservableCacheFixture.cs`):
//! the children of parents as one cache, followed as the collections of children change.

use crate::domain::Person;
use ferroui_dynamic_data::binding::{ObservableCollectionEx, ReadOnlyObservableCollection};
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `Parent`: the children in an observable collection, its read-only view, and the observable
/// cache of the children keyed by name.
struct Parent {
    id: i32,
    children: ObservableCollection<Rc<Person>>,
    children_readonly: ReadOnlyObservableCollection<Rc<Person>>,
    children_cache: ObservableCache<Rc<Person>, String>,
}

impl Parent {
    /// `Parent(int id, IEnumerable<Person> children)`; `Parent(int id)` without children.
    fn new(id: i32, children: Vec<Rc<Person>>) -> Rc<Self> {
        let children = ObservableCollection::from_items(children);
        let children_readonly = ReadOnlyObservableCollection::new(children.clone());
        let children_cache = children.to_observable_change_set_with_key(|x: &Rc<Person>| x.name()).as_observable_cache();
        Rc::new(Self { id, children, children_readonly, children_cache })
    }
}

/// `ObservableCollection.AddRange`: the items one by one.
fn add_range(collection: &ObservableCollection<Rc<Person>>, items: impl IntoIterator<Item = Rc<Person>>) {
    for item in items {
        collection.add(item);
    }
}

/// The fifty parents of two children each.
fn make_parents() -> Vec<Rc<Parent>> {
    let children: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();
    let mut child_index = 0;
    (1..=50)
        .map(|i| {
            let parent = Parent::new(i, vec![children[child_index].clone(), children[child_index + 1].clone()]);
            child_index += 2;
            parent
        })
        .collect()
}

/// The steps that every `Flatten*` test takes, through the children that `many_selector` selects.
fn flatten(make_aggregator: impl FnOnce(&SourceCache<Rc<Parent>, i32>) -> ferroui_dynamic_data::cache::testing::ChangeSetAggregator<Rc<Person>, String>) {
    let parents = make_parents();

    let source = SourceCache::new(|x: &Rc<Parent>| x.id);
    let aggregator = make_aggregator(&source);
    source.add_or_update_items(parents.iter().cloned());

    assert_eq!(aggregator.data().count(), 100);

    // Add a child to an observable collection and check the new item is added.
    parents[0].children.add(Person::new("NewlyAddded", 100));
    assert_eq!(aggregator.data().count(), 101);

    // Remove the first parent and check its children have gone.
    source.remove_key(1);
    assert_eq!(aggregator.data().count(), 98);

    // Check items can be cleared and then added back in.
    let children_in_zero = parents[1].children.to_vec();
    parents[1].children.clear();
    assert_eq!(aggregator.data().count(), 96);
    add_range(&parents[1].children, children_in_zero);
    assert_eq!(aggregator.data().count(), 98);

    // Replace produces an update.
    let replaced_child = parents[1].children.get(0);
    parents[1].children.set(0, Person::new("Replacement", 100));
    assert_eq!(aggregator.data().count(), 98);

    assert!(aggregator.data().lookup(&replaced_child.key()).is_none());
    assert!(aggregator.data().lookup(&"Replacement".to_owned()).is_some());

    aggregator.dispose();
    source.dispose();
}

#[test]
fn flatten_observable_collection() {
    flatten(|source| source.connect().transform_many_observable_collection(|p: &Rc<Parent>| p.children.clone(), |c: &Rc<Person>| c.name()).as_aggregator());
}

#[test]
fn flatten_read_only_observable_collection() {
    flatten(|source| source.connect().transform_many_observable_collection(|p: &Rc<Parent>| p.children_readonly.clone(), |c: &Rc<Person>| c.name()).as_aggregator());
}

#[test]
fn flatten_observable_cache() {
    flatten(|source| source.connect().transform_many_observable_cache(|p: &Rc<Parent>| p.children_cache.clone(), |c: &Rc<Person>| c.name()).as_aggregator());
}

/// The steps that every `*WithoutInitialData` test takes.
fn without_initial_data(make_collection: impl FnOnce(&SourceCache<Rc<Parent>, i32>) -> ObservableCache<Rc<Person>, String>) {
    let parents = SourceCache::new(|d: &Rc<Parent>| d.id);
    let collection = make_collection(&parents);

    let parent = Parent::new(1, Vec::new());
    parents.add_or_update(parent.clone());

    assert_eq!(collection.count(), 0);

    parent.children.add(Person::new("child1", 1));
    assert_eq!(collection.count(), 1);

    parent.children.add(Person::new("child2", 2));
    assert_eq!(collection.count(), 2);

    parents.dispose();
}

#[test]
fn observable_collection_without_initial_data() {
    without_initial_data(|parents| parents.connect().transform_many_observable_collection(|d: &Rc<Parent>| d.children.clone(), |p: &Rc<Person>| p.name()).as_observable_cache());
}

#[test]
fn read_only_observable_collection_without_initial_data() {
    without_initial_data(|parents| parents.connect().transform_many_observable_collection(|d: &Rc<Parent>| d.children_readonly.clone(), |p: &Rc<Person>| p.name()).as_observable_cache());
}

#[test]
fn observable_cache_without_initial_data() {
    without_initial_data(|parents| parents.connect().transform_many_observable_cache(|d: &Rc<Parent>| d.children_cache.clone(), |p: &Rc<Person>| p.name()).as_observable_cache());
}
