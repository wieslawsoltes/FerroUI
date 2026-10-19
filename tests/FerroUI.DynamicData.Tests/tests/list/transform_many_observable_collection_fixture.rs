//! `TransformManyObservableCollectionFixture` (`DynamicData.Tests/List/TransformManyObservableCollectionFixture.cs`):
//! the children of parents as one list, followed as the collections of children change.

use crate::domain::Person;
use ferroui_dynamic_data::list::{IObservableList, ObservableList, ObservableListEx, SourceList};
use ferroui_system::IDisposable;
use std::rc::Rc;

// Not ported: FlattenObservableCollection - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: FlattenReadOnlyObservableCollection - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: ObservableCollectionWithoutInitialData - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: ReadOnlyObservableCollectionWithoutInitialData - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)

/// `ParentDynamic`: the children in a source list, and its observable list.
struct ParentDynamic {
    children: SourceList<Rc<Person>>,
    children_observable: ObservableList<Rc<Person>>,
}

impl PartialEq for ParentDynamic {
    /// The same parent (reference equality, as upstream's class).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ParentDynamic {
    fn new(children: Vec<Rc<Person>>) -> Rc<Self> {
        let source = SourceList::new();
        source.edit(|list| list.add_range(children));
        let children_observable = ObservableList::from_list(&source);
        Rc::new(Self { children: source, children_observable })
    }

    fn add_child(&self, child: Rc<Person>) {
        self.children.edit(|list| list.add(child));
    }
}

#[test]
fn flatten_observable_list() {
    let children: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();

    let parents: Vec<Rc<ParentDynamic>> = (0..50).map(|i| ParentDynamic::new(vec![children[i * 2].clone(), children[i * 2 + 1].clone()])).collect();

    let source = SourceList::<Rc<ParentDynamic>>::new();
    let aggregator = source.connect().transform_many_observable_list(|p: &Rc<ParentDynamic>| p.children_observable.clone()).as_aggregator();

    source.edit(|list| list.add_range(parents.clone()));

    assert_eq!(aggregator.data().count(), 100);

    // Add a child to an observable collection and check the new item is added.
    parents[0].add_child(Person::new("NewlyAddded", 100));
    assert_eq!(aggregator.data().count(), 101);

    // Remove the first parent and check its children have gone.
    source.edit(|list| list.remove_at(0));
    assert_eq!(aggregator.data().count(), 98);

    // Check items can be cleared and then added back in.
    let children_in_zero = parents[1].children.items();
    parents[1].children.edit(|list| list.clear());
    assert_eq!(aggregator.data().count(), 96);
    parents[1].children.edit(|list| list.add_range(children_in_zero));
    assert_eq!(aggregator.data().count(), 98);

    // Replace produces an update.
    let replaced_child = parents[1].children.items()[0].clone();
    let replacement = Person::new("Replacement", 100);
    parents[1].children.edit(|list| list.set(0, replacement.clone()));
    assert_eq!(aggregator.data().count(), 98);

    // Replace produces an update.
    assert!(aggregator.data().items().contains(&replacement));
    assert!(!aggregator.data().items().contains(&replaced_child));

    source.dispose();
}

#[test]
fn observable_list_without_initial_data() {
    let parents = SourceList::<Rc<ParentDynamic>>::new();
    let collection = parents.connect().transform_many_observable_list(|d: &Rc<ParentDynamic>| d.children_observable.clone()).as_observable_list();

    let parent = ParentDynamic::new(Vec::new());
    parents.edit(|list| list.add(parent.clone()));

    assert_eq!(collection.count(), 0);

    parent.add_child(Person::new("child1", 1));
    assert_eq!(collection.count(), 1);

    parent.add_child(Person::new("child2", 2));
    assert_eq!(collection.count(), 2);

    parents.dispose();
}
