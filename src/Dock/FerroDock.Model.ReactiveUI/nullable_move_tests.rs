//! Original null-valued move/swap operations, callback order and membership dispatch.
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    collections::ListHandle, controls::IRootDock, core::*, DockCapabilityResolver, FactoryBase,
    FactoryBaseImpl,
};
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct MoveFactory {
    component: Factory,
    trace: Rc<RefCell<Vec<String>>>,
    root: RefCell<Option<Rc<dyn IRootDock>>>,
}
fn name(item: &Option<Rc<dyn IDockable>>) -> String {
    item.as_ref().map_or_else(|| "null".into(), |x| x.id())
}
fn item(id: &str) -> Rc<dyn IDockable> {
    let item = Document::new();
    item.set_id(id.into());
    item
}
fn shape(list: &ListHandle<Option<Rc<dyn IDockable>>>) -> String {
    list.iter().map(|i| name(&i)).collect::<Vec<_>>().join(",")
}
impl MoveFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: Default::default(),
            root: Default::default(),
        })
    }
}
impl FactoryBaseImpl for MoveFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for MoveFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        this.trace
            .borrow_mut()
            .push(format!("root:{}", name(&item)));
        this.root.borrow().clone()
    }
    fn pin_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        this.trace.borrow_mut().push(format!("pin:{}", name(&item)));
    }
    fn remove_dockable(this: &Self, item: Option<Rc<dyn IDockable>>, _: bool) {
        this.trace
            .borrow_mut()
            .push(format!("remove:{}", name(&item)));
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.trace
            .borrow_mut()
            .push(format!("init:{}:{}", name(&item), name(&owner)));
    }
}
impl_factory!(MoveFactory);
fn events(factory: &Rc<MoveFactory>) {
    macro_rules! event {
        ($method:ident,$label:literal) => {{
            let trace = factory.trace.clone();
            factory.$method(Rc::new(move |args| {
                trace
                    .borrow_mut()
                    .push(format!("{}:{}", $label, name(&args.dockable())))
            }));
        }};
    }
    event!(dockable_added, "added");
    event!(dockable_removed, "removed");
    event!(dockable_moved, "moved");
    event!(dockable_swapped, "swapped");
    event!(dockable_undocked, "undocked");
    event!(dockable_docked, "docked");
}
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn same_dock_move_keeps_null_entry_list_identity_selection_and_original_event_order() {
    let f = MoveFactory::new();
    events(&f);
    let dock = DocumentDock::new();
    let a = item("a");
    let b = item("b");
    let list = f.create_list(vec![None, Some(a.clone()), Some(b.clone())]);
    dock.set_visible_dockables(Some(list.clone()));
    f.move_dockable(Some(dock.clone()), None, Some(b.clone()));
    assert_eq!(shape(&list), "a,b,null");
    assert!(list.ptr_eq(&dock.visible_dockables().unwrap()));
    assert!(dock.active_dockable().is_none());
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "removed:null",
            "undocked:null",
            "added:null",
            "moved:null",
            "docked:null"
        ]
    );
    f.trace.borrow_mut().clear();
    f.move_dockable(Some(dock.clone()), Some(b.clone()), None);
    assert_eq!(shape(&list), "a,null,b");
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &b));
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "removed:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "undocked:b",
            "added:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "moved:b",
            "docked:b"
        ]
    );
    f.trace.borrow_mut().clear();
    f.move_dockable(Some(dock.clone()), None, None);
    assert!(f.trace.borrow().is_empty());
    assert!(failure(|| f.move_dockable(None, None, None))
        .contains("move_dockable dock.VisibleDockables"));
    f.move_dockable(Some(DocumentDock::new()), None, None);
}
#[test]
fn same_dock_swap_uses_original_nullable_values_despite_removed_callback_reentry() {
    let f = MoveFactory::new();
    events(&f);
    let dock = DocumentDock::new();
    let b = item("b");
    let c = item("c");
    let list = f.create_list(vec![None, Some(b.clone())]);
    dock.set_visible_dockables(Some(list.clone()));
    let during = list.clone();
    let trace = f.trace.clone();
    let handler = f.dockable_removed(Rc::new(move |args| {
        assert_eq!(args.dockable().as_ref().unwrap().id(), "b");
        assert_eq!(shape(&during), "null,null");
        during.set(0, Some(c.clone()));
        trace.borrow_mut().push("reentered".into());
    }));
    f.swap_dockable(Some(dock.clone()), None, Some(b.clone()));
    handler.dispose();
    assert_eq!(shape(&list), "b,null");
    assert!(list.ptr_eq(&dock.visible_dockables().unwrap()));
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &b));
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "removed:b",
            "reentered",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "added:null",
            "added:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "root:b",
            "swapped:null",
            "swapped:b"
        ]
    );
    f.trace.borrow_mut().clear();
    f.swap_dockable(Some(dock), None, None);
    assert!(f.trace.borrow().is_empty());
    assert!(failure(|| f.swap_dockable(None, None, None))
        .contains("swap_dockable dock.VisibleDockables"));
}
#[test]
fn cross_swap_commits_lists_before_default_null_init_failure_or_custom_nullable_init() {
    for custom in [true, false] {
        let f = MoveFactory::new();
        events(&f);
        let source = DocumentDock::new();
        source.set_id("source".into());
        let target = DocumentDock::new();
        target.set_id("target".into());
        let b = item("b");
        let left = f.create_list(vec![None]);
        let right = f.create_list(vec![Some(b.clone())]);
        source.set_visible_dockables(Some(left.clone()));
        target.set_visible_dockables(Some(right.clone()));
        if custom {
            f.swap_dockable_between_docks(
                Some(source.clone()),
                Some(target.clone()),
                None,
                Some(b.clone()),
            );
            assert_eq!(
                &*f.trace.borrow(),
                &[
                    "init:null:target",
                    "init:b:source",
                    "swapped:b",
                    "swapped:null"
                ]
            );
            assert!(Rc::ptr_eq(&source.active_dockable().unwrap(), &b));
            assert!(target.active_dockable().is_none());
        } else {
            assert!(failure(|| Factory::new().swap_dockable_between_docks(
                Some(source.clone()),
                Some(target.clone()),
                None,
                Some(b.clone())
            ))
            .contains("dockable.Context"));
            assert!(b.factory().is_none());
            assert!(source.active_dockable().is_none());
        }
        assert_eq!(shape(&left), "b");
        assert_eq!(shape(&right), "null");
        assert!(left.ptr_eq(&source.visible_dockables().unwrap()));
        assert!(right.ptr_eq(&target.visible_dockables().unwrap()));
    }
    let f = MoveFactory::new();
    events(&f);
    let dock = DocumentDock::new();
    dock.set_id("same".into());
    dock.set_visible_dockables(Some(f.create_list(vec![None])));
    f.swap_dockable_between_docks(Some(dock.clone()), Some(dock), None, None);
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "init:null:same",
            "init:null:same",
            "swapped:null",
            "swapped:null"
        ]
    );
}
#[test]
fn cross_move_preserves_unpin_insert_before_remove_same_owner_order_and_custom_remove() {
    let f = MoveFactory::new();
    events(&f);
    let source = DocumentDock::new();
    let a = item("a");
    let b = item("b");
    let left = f.create_list(vec![None, Some(a.clone()), Some(b.clone())]);
    source.set_visible_dockables(Some(left.clone()));
    source.set_active_dockable(Some(a.clone()));
    f.move_dockable_between_docks(Some(source.clone()), Some(source.clone()), None, Some(b));
    assert_eq!(shape(&left), "a,b,null");
    assert!(Rc::ptr_eq(&source.active_dockable().unwrap(), &a));
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "root:null",
            "added:null",
            "removed:null",
            "undocked:null",
            "moved:null",
            "docked:null"
        ]
    );
    f.trace.borrow_mut().clear();
    let target = DocumentDock::new();
    target.set_id("target".into());
    let right = f.create_list(vec![]);
    target.set_visible_dockables(Some(right.clone()));
    f.move_dockable_between_docks(Some(source), Some(target.clone()), None, None);
    assert_eq!(shape(&left), "a,b,null");
    assert_eq!(shape(&right), "null");
    assert!(right.ptr_eq(&target.visible_dockables().unwrap()));
    assert_eq!(target.opened_dockables_count(), 1);
    assert!(target.is_empty());
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "root:null",
            "remove:null",
            "undocked:null",
            "init:null:target",
            "added:null",
            "moved:null",
            "docked:null"
        ]
    );
}
#[test]
fn nullable_dock_order_preserves_unpin_target_creation_and_swap_short_circuit() {
    let f = MoveFactory::new();
    assert!(
        failure(|| f.move_dockable_between_docks(None, None, None, None))
            .contains("move target.VisibleDockables")
    );
    assert_eq!(&*f.trace.borrow(), &["root:null"]);
    f.trace.borrow_mut().clear();
    let target = DocumentDock::new();
    assert!(
        failure(|| f.move_dockable_between_docks(None, Some(target.clone()), None, None))
            .contains("move source.VisibleDockables")
    );
    assert_eq!(target.visible_dockables().unwrap().count(), 0);
    assert_eq!(&*f.trace.borrow(), &["root:null"]);
    f.trace.borrow_mut().clear();
    let source = DocumentDock::new();
    f.swap_dockable_between_docks(Some(source.clone()), None, None, None);
    assert!(f.trace.borrow().is_empty());
    source.set_visible_dockables(Some(f.create_list(vec![])));
    assert!(
        failure(|| f.swap_dockable_between_docks(Some(source), None, None, None))
            .contains("swap target.VisibleDockables")
    );
    assert!(
        failure(|| f.swap_dockable_between_docks(None, None, None, None))
            .contains("swap source.VisibleDockables")
    );
}
#[test]
fn pinned_move_null_uses_first_original_strip_and_keeps_query_and_event_identity() {
    let f = MoveFactory::new();
    events(&f);
    let root = RootDock::new();
    let a = item("a");
    let pins = f.create_list(vec![None, Some(a.clone())]);
    root.set_left_pinned_dockables(Some(pins.clone()));
    f.root.replace(Some(root.clone()));
    f.move_pinned_dockable(None, Some(a.clone()));
    assert_eq!(shape(&pins), "a,null");
    assert!(pins.ptr_eq(&root.left_pinned_dockables().unwrap()));
    assert_eq!(&*f.trace.borrow(), &["root:null", "moved:null"]);
    f.trace.borrow_mut().clear();
    root.set_left_pinned_dockables(Some(f.create_list(vec![None])));
    root.set_right_pinned_dockables(Some(f.create_list(vec![Some(a.clone())])));
    f.move_pinned_dockable(None, Some(a));
    assert_eq!(&*f.trace.borrow(), &["root:null"]);
    f.root.replace(None);
    f.trace.borrow_mut().clear();
    f.move_pinned_dockable(None, None);
    assert_eq!(&*f.trace.borrow(), &["root:null"]);
}
#[test]
fn unpin_null_preserves_fixed_membership_before_virtual_pin_and_default_failure_order() {
    let f = MoveFactory::new();
    f.unpin_dockable(None);
    assert_eq!(&*f.trace.borrow(), &["root:null"]);
    f.trace.borrow_mut().clear();
    let root = RootDock::new();
    let list = f.create_list(vec![None]);
    root.set_left_pinned_dockables(Some(list.clone()));
    f.root.replace(Some(root));
    f.unpin_dockable(None);
    assert_eq!(&*f.trace.borrow(), &["root:null", "pin:null"]);
    assert_eq!(list.count(), 1);
    assert!(failure(|| Factory::new().unpin_dockable(None)).contains("dockable.Owner"));
    assert!(failure(|| Factory::new().pin_dockable(None))
        .contains("ResolveOperationDock dockable.Owner"));
}
#[test]
fn nullable_capability_entry_points_fail_at_original_operation_or_flag_access() {
    assert!(failure(|| {
        DockCapabilityResolver::evaluate(None, DockCapability::Pin);
    })
    .contains("ResolveOperationDock dockable.Owner"));
    assert!(failure(|| {
        DockCapabilityResolver::evaluate_with_context(None, DockCapability::Pin, None);
    })
    .contains("capability dockable flag"));
    assert!(failure(|| {
        DockCapabilityResolver::is_enabled(None, DockCapability::Pin, Some(DocumentDock::new()));
    })
    .contains("capability dockable flag"));
    assert!(failure(|| {
        DockCapabilityResolver::resolve_drop_target_dock(None);
    })
    .contains("ResolveDropTargetDock target.Owner"));
}
