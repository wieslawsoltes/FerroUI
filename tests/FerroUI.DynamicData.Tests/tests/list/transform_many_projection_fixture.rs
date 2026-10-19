//! `TransformManyProjectionFixture` (`DynamicData.Tests/List/TransformManyProjectionFixture.cs`): the
//! children of parents projected with their parent, as one list.
//!
//! The fixture refreshes each parent on the changes of its `ObservableCollection<NestedChild>`,
//! through `AutoRefreshOnObservable(self => self.Children.ToObservableChangeSet())`.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::ObservableCollectionEx;
use ferroui_dynamic_data::list::{IObservableList, ObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `ClassWithNestedObservableCollection`.
struct ClassWithNestedObservableCollection {
    children: ObservableCollection<Rc<NestedChild>>,
    id: i32,
}

impl ClassWithNestedObservableCollection {
    fn new(id: i32, animals: Vec<Rc<NestedChild>>) -> Rc<Self> {
        Rc::new(Self { children: ObservableCollection::from_items(animals), id })
    }
}

impl PartialEq for ClassWithNestedObservableCollection {
    /// The same object (reference equality, as upstream's class).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `NestedChild`.
struct NestedChild {
    name: String,
    #[allow(dead_code)]
    value: String,
}

impl NestedChild {
    fn new(name: &str, value: &str) -> Rc<Self> {
        Rc::new(Self { name: name.to_owned(), value: value.to_owned() })
    }
}

impl PartialEq for NestedChild {
    /// The same object (reference equality, as upstream's class).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `ProjectedNestedChild`.
struct ProjectedNestedChild {
    child: Rc<NestedChild>,
    parent: Rc<ClassWithNestedObservableCollection>,
}

impl ProjectedNestedChild {
    fn new(parent: Rc<ClassWithNestedObservableCollection>, child: Rc<NestedChild>) -> Rc<Self> {
        Rc::new(Self { child, parent })
    }
}

impl PartialEq for ProjectedNestedChild {
    /// The same object (reference equality, as upstream's class).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `ProjectNestedChildEqualityComparer`: the same name of the child.
fn project_nested_child_equality_comparer(x: &Rc<ProjectedNestedChild>, y: &Rc<ProjectedNestedChild>) -> bool {
    x.child.name == y.child.name
}

/// The structure of a projection, which FluentAssertions' `BeEquivalentTo` compares member by
/// member: the identifier of the parent and the name of the child (each object once).
fn structure(projections: &[Rc<ProjectedNestedChild>]) -> Vec<(i32, String)> {
    projections.iter().map(|projection| (projection.parent.id, projection.child.name.clone())).collect()
}

struct Fixture {
    results: ObservableList<Rc<ProjectedNestedChild>>,
    source: SourceList<Rc<ClassWithNestedObservableCollection>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();

        let results = source
            .connect()
            .auto_refresh_on_observable(|s: &Rc<ClassWithNestedObservableCollection>| s.children.to_observable_change_set())
            .transform_many_with_comparer(
                |parent: &Rc<ClassWithNestedObservableCollection>| parent.children.to_vec().into_iter().map(|c| ProjectedNestedChild::new(parent.clone(), c)).collect::<Vec<_>>(),
                project_nested_child_equality_comparer,
            )
            .as_observable_list();
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

/// The children and the parents of every test.
fn children_and_parents() -> (Vec<Rc<NestedChild>>, Vec<Rc<ClassWithNestedObservableCollection>>) {
    let children = vec![
        NestedChild::new("A", "ValueA"),
        NestedChild::new("B", "ValueB"),
        NestedChild::new("C", "ValueC"),
        NestedChild::new("D", "ValueD"),
        NestedChild::new("E", "ValueE"),
        NestedChild::new("F", "ValueF"),
    ];

    let parents = vec![
        ClassWithNestedObservableCollection::new(1, vec![children[0].clone(), children[1].clone()]),
        ClassWithNestedObservableCollection::new(2, vec![children[2].clone(), children[3].clone()]),
        ClassWithNestedObservableCollection::new(3, vec![children[4].clone()]),
    ];
    (children, parents)
}

/// The projections of the children of `parents` that `include` accepts.
fn projections(parents: &[Rc<ClassWithNestedObservableCollection>], include: impl Fn(&Rc<NestedChild>) -> bool) -> Vec<Rc<ProjectedNestedChild>> {
    parents
        .iter()
        .flat_map(|p| p.children.to_vec().into_iter().filter(|c| include(c)).map(|c| ProjectedNestedChild::new(p.clone(), c)).collect::<Vec<_>>())
        .collect()
}

#[test]
fn add_range() {
    let fixture = Fixture::new();
    let (_children, parents) = children_and_parents();

    fixture.source.add_range(parents.clone());

    assert_eq!(fixture.results.count(), 5);
    // `p.Children.Take(5)`: every child, as no parent has more than two.
    assert_equivalent(&structure(&fixture.results.items()), &structure(&projections(&parents, |_| true)));
}

#[test]
fn remove_child() {
    let fixture = Fixture::new();
    let (children, parents) = children_and_parents();

    fixture.source.add_range(parents.clone());

    // Remove a child.
    parents[1].children.remove(&children[3]);
    assert_eq!(fixture.results.count(), 4);
    assert_equivalent(&structure(&fixture.results.items()), &structure(&projections(&parents, |child| child.name != "D")));
}

#[test]
fn remove_parent() {
    let fixture = Fixture::new();
    let (_children, parents) = children_and_parents();

    fixture.source.add_range(parents.clone());

    // Remove a parent and check children have moved.
    fixture.source.remove(&parents[0]);
    assert_eq!(fixture.results.count(), 3);
    assert_equivalent(&structure(&fixture.results.items()), &structure(&projections(&parents[1..], |_| true)));
}
