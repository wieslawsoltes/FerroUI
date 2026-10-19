//! `TransformTreeWithRefreshFixture` (`DynamicData.Tests/Cache/TransformTreeWithRefreshFixture.cs`):
//! the tree of bosses as the bosses of the employees change.

use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, Node, ObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// `EmployeeDto`: `BossId` and `Name` raise `PropertyChanged`; equal by its id; held in `Rc`.
struct EmployeeDto {
    notify: AbstractNotifyPropertyChanged,
    boss_id: Cell<i32>,
    id: i32,
    name: RefCell<Option<String>>,
}

impl EmployeeDto {
    fn new(id: i32, boss_id: i32, name: &str) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), boss_id: Cell::new(boss_id), id, name: RefCell::new(Some(name.to_owned())) })
    }

    fn boss_id(&self) -> i32 {
        self.boss_id.get()
    }

    fn set_boss_id(&self, value: i32) {
        self.notify.set_and_raise(&self.boss_id, value, "BossId");
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: &str) {
        self.notify.set_and_raise(&self.name, Some(value.to_owned()), "Name");
    }
}

impl INotifyPropertyChanged for EmployeeDto {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
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
    result: ObservableCache<Rc<Node<Rc<EmployeeDto>, i32>>, i32>,
    source_cache: SourceCache<Rc<EmployeeDto>, i32>,
}

impl Fixture {
    fn new() -> Self {
        let source_cache = SourceCache::new(|e: &Rc<EmployeeDto>| e.id());
        let result = source_cache.connect().auto_refresh_any().transform_to_tree(|e: &Rc<EmployeeDto>| e.boss_id()).as_observable_cache();
        source_cache.edit(|updater| updater.add_or_update_items(&mut create_employees().into_iter()));
        Self { result, source_cache }
    }

    fn employee(&self, id: i32) -> Rc<EmployeeDto> {
        self.source_cache.lookup(&id).expect("the employee is in the cache")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source_cache.dispose();
        self.result.dispose();
    }
}

#[test]
fn do_not_update_tree_when_parent_id_not_changed() {
    let fixture = Fixture::new();
    fixture.employee(1).set_name("Employee11");
    fixture.employee(2).set_name("Employee22");

    let node1 = fixture.result.lookup(&1);
    assert!(node1.is_some());
    let node1 = node1.unwrap();
    assert!(node1.parent().is_none());
    let node2 = node1.children().lookup(&2);
    assert!(node2.is_some());
    let node2 = node2.unwrap();
    assert!(node2.parent().is_some());
    assert_eq!(*node2.parent().unwrap().key(), 1);
}

#[test]
fn update_tree_when_parent_id_of_non_root_item_changed_to_existing_id() {
    let fixture = Fixture::new();
    fixture.employee(2).set_boss_id(3);

    // Node 2 added to the children of node 3.
    let node2 = fixture.result.lookup(&1).unwrap().children().lookup(&3).unwrap().children().lookup(&2);
    assert!(node2.is_some());
    assert!(!node2.unwrap().is_root());

    // Node 2 removed from the children of node 1.
    assert!(fixture.result.lookup(&1).unwrap().children().lookup(&2).is_none());
}

#[test]
fn update_tree_when_parent_id_of_non_root_item_changed_to_non_existing_id() {
    let fixture = Fixture::new();
    fixture.employee(2).set_boss_id(25);

    // Node 2 added to the roots.
    let node2 = fixture.result.lookup(&2);
    assert!(node2.is_some());
    assert!(node2.unwrap().is_root());

    // Node 2 removed from the children of node 1.
    assert!(fixture.result.lookup(&1).unwrap().children().lookup(&2).is_none());
}

#[test]
fn update_tree_when_parent_id_of_root_item_changed_to_existing_id() {
    let fixture = Fixture::new();
    fixture.employee(1).set_boss_id(7);

    // Node 1 added to the children of node 7.
    let node1 = fixture.result.lookup(&7).unwrap().children().lookup(&1);
    assert!(node1.is_some());
    assert!(!node1.unwrap().is_root());

    // Node 1 removed from the roots.
    assert!(fixture.result.lookup(&1).is_none());
}

#[test]
fn update_tree_when_parent_id_of_root_item_changed_to_non_existing_id() {
    let fixture = Fixture::new();
    fixture.employee(1).set_boss_id(25);

    // Node 1 is still a root.
    let node1 = fixture.result.lookup(&1);
    assert!(node1.is_some());
    assert!(node1.unwrap().is_root());
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
