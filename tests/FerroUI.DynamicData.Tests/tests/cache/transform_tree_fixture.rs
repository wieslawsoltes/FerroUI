//! `TransformTreeFixture` (`DynamicData.Tests/Cache/TransformTreeFixture.cs`): employees as the
//! nodes of a tree of bosses.

use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, Node, ObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObserver};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

type NodePredicate = Rc<dyn Fn(&Rc<Node<Rc<EmployeeDto>, i32>>) -> bool>;

/// `EmployeeDto`: equal by its id; held in `Rc`.
struct EmployeeDto {
    boss_id: Cell<i32>,
    id: i32,
    name: RefCell<Option<String>>,
}

impl EmployeeDto {
    fn new(id: i32, boss_id: i32, name: &str) -> Rc<Self> {
        Rc::new(Self { boss_id: Cell::new(boss_id), id, name: RefCell::new(Some(name.to_owned())) })
    }

    fn boss_id(&self) -> i32 {
        self.boss_id.get()
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }
}

impl PartialEq for EmployeeDto {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.id == other.id
    }
}

impl fmt::Display for EmployeeDto {
    /// `ToString`: `Name: {Name}, Id: {Id}, BossId: {BossId}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, Id: {}, BossId: {}", self.name().unwrap_or_default(), self.id, self.boss_id())
    }
}

impl fmt::Debug for EmployeeDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

struct Fixture {
    filter: Rc<BehaviorSubject<NodePredicate>>,
    result: ObservableCache<Rc<Node<Rc<EmployeeDto>, i32>>, i32>,
    source_cache: SourceCache<Rc<EmployeeDto>, i32>,
}

impl Fixture {
    fn new() -> Self {
        let source_cache = SourceCache::new(|e: &Rc<EmployeeDto>| e.id());

        let is_root: NodePredicate = Rc::new(|n: &Rc<Node<Rc<EmployeeDto>, i32>>| n.is_root());
        let filter = BehaviorSubject::new(is_root);

        let result = source_cache.connect().transform_to_tree_with(|e: &Rc<EmployeeDto>| e.boss_id(), filter.as_observable()).as_observable_cache();
        Self { filter, result, source_cache }
    }

    fn add_or_update(&self, employees: Vec<Rc<EmployeeDto>>) {
        self.source_cache.edit(|updater| updater.add_or_update_items(&mut employees.into_iter()));
    }

    fn set_filter(&self, predicate: impl Fn(&Rc<Node<Rc<EmployeeDto>, i32>>) -> bool + 'static) {
        let predicate: NodePredicate = Rc::new(predicate);
        self.filter.on_next(predicate);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source_cache.dispose();
        self.result.dispose();
        self.filter.dispose();
    }
}

#[test]
fn add_missing_child() {
    let fixture = Fixture::new();
    let boss = EmployeeDto::new(2, 0, "Boss");
    let minion = EmployeeDto::new(1, 2, "DogsBody");
    fixture.add_or_update(vec![boss.clone()]);
    fixture.add_or_update(vec![minion.clone()]);

    assert_eq!(fixture.result.count(), 1);

    let first_node = fixture.result.items()[0].clone();
    assert_eq!(*first_node.item(), boss);

    let child_node = first_node.children().items()[0].clone();
    assert_eq!(*child_node.item(), minion);
}

#[test]
fn add_missing_parent() {
    let fixture = Fixture::new();
    let emp10 = EmployeeDto::new(10, 11, "Employee10");
    let emp11 = EmployeeDto::new(11, 0, "Employee11");
    let emp12 = EmployeeDto::new(12, 13, "Employee12");
    let emp13 = EmployeeDto::new(13, 11, "Employee13");

    fixture.add_or_update(vec![emp10]);
    fixture.add_or_update(vec![emp11]);
    fixture.add_or_update(vec![emp12]);
    fixture.add_or_update(vec![emp13]);

    assert_eq!(fixture.result.count(), 1);

    let emp11_node = fixture.result.lookup(&11);
    assert!(emp11_node.is_some());
    let emp11_node = emp11_node.unwrap();
    assert_eq!(emp11_node.children().count(), 2);

    let emp10_node = emp11_node.children().lookup(&10);
    assert!(emp10_node.is_some());
    assert_eq!(emp10_node.unwrap().children().count(), 0);

    let emp13_node = emp11_node.children().lookup(&13);
    assert!(emp13_node.is_some());
    let emp13_node = emp13_node.unwrap();
    assert_eq!(emp13_node.children().count(), 1);

    let emp12_node = emp13_node.children().lookup(&12);
    assert!(emp12_node.is_some());
    assert_eq!(emp12_node.unwrap().children().count(), 0);
}

#[test]
fn build_tree_from_mixed_data() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());
    assert_eq!(fixture.result.count(), 2);

    let first_node = fixture.result.items()[0].clone();
    assert_eq!(first_node.children().count(), 3);

    let second_node = fixture.result.items()[1].clone();
    assert_eq!(second_node.children().count(), 0);
}

#[test]
fn change_parent() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());

    fixture.add_or_update(vec![EmployeeDto::new(4, 1, "Employee4")]);

    // If this fails, then employee 4 is not a child of boss 1.
    let emp4 = fixture.result.lookup(&1).unwrap().children().lookup(&4).unwrap();

    // Check the boss is 1.
    assert_eq!(emp4.parent().unwrap().item().id(), 1);

    // Look up the previous boss (employee 4 should no longer be a child).
    let emp3 = fixture.result.lookup(&1).unwrap().children().lookup(&3).unwrap();

    // Employee 4 must be removed from the child collection of the previous boss.
    assert!(emp3.children().lookup(&4).is_none());
}

#[test]
fn remove_a_child_node_will_push_orphans_up_the_hierachy() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());
    fixture.source_cache.edit(|updater| updater.remove_key(&4));

    // The children of node 4 are expected to be pushed up and become new roots.
    assert_eq!(fixture.result.count(), 3);

    let third_node = fixture.result.items()[2].clone();
    assert_eq!(*third_node.key(), 5);
}

#[test]
fn remove_a_root_node_will_push_orphans_up_the_hierachy() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());
    fixture.source_cache.edit(|updater| updater.remove_key(&1));

    // The original children nodes are expected to be pushed up and become new roots.
    assert_eq!(fixture.result.count(), 4);
}

#[test]
fn update_a_parent_node() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());

    let changed = EmployeeDto::new(1, 0, "Employee 1 (with name change)");

    fixture.add_or_update(vec![changed.clone()]);
    assert_eq!(fixture.result.count(), 2);

    let first_node = fixture.result.items()[0].clone();
    assert_eq!(first_node.children().count(), 3);
    assert_eq!(first_node.item().name(), changed.name());
}

#[test]
fn update_child_node() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());

    let changed = EmployeeDto::new(2, 1, "Employee 2 (with name change)");

    fixture.add_or_update(vec![changed.clone()]);
    assert_eq!(fixture.result.count(), 2);

    let changed_node = fixture.result.items()[0].children().items()[0].clone();

    assert_eq!(changed_node.parent().unwrap().item().id(), 1);
    assert_eq!(changed_node.children().count(), 1);
    assert_eq!(changed.name(), changed.name());
}

#[test]
fn use_custom_filter() {
    let fixture = Fixture::new();
    fixture.add_or_update(create_employees());

    assert_eq!(fixture.result.count(), 2);

    fixture.set_filter(|_| true);
    assert_eq!(fixture.result.count(), 8);

    fixture.set_filter(|node| node.depth() == 3);
    assert_eq!(fixture.result.count(), 1);

    fixture.source_cache.edit(|updater| updater.remove_key(&5));
    assert_eq!(fixture.result.count(), 0);

    fixture.set_filter(|node| node.is_root());
    assert_eq!(fixture.result.count(), 2);
}

fn create_employees() -> Vec<Rc<EmployeeDto>> {
    vec![
        EmployeeDto::new(1, 0, "Employee1"),
        EmployeeDto::new(2, 1, "Employee2"),
        EmployeeDto::new(3, 1, "Employee3"),
        EmployeeDto::new(4, 3, "Employee4"),
        EmployeeDto::new(5, 4, "Employee5"),
        EmployeeDto::new(6, 2, "Employee6"),
        EmployeeDto::new(7, 0, "Employee7"),
        EmployeeDto::new(8, 1, "Employee8"),
    ]
}
