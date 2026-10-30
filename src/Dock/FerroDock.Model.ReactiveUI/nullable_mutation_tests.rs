//! Original nullable insertion/removal/pin-cleanup and floating override order.
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    controls::{IRootDock, IToolDock},
    core::*,
    FactoryBase, FactoryBaseImpl,
};
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct MutationFactory {
    component: Factory,
    trace: Rc<RefCell<Vec<String>>>,
    null_root: RefCell<Option<Rc<dyn IRootDock>>>,
    pin_root_base: Cell<bool>,
    removal_noop: Cell<bool>,
    floated: RefCell<Option<Rc<dyn IDock>>>,
}
fn label(item: &Option<Rc<dyn IDockable>>) -> String {
    item.as_ref()
        .map_or_else(|| "null".into(), |item| item.id())
}
impl MutationFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: Default::default(),
            null_root: Default::default(),
            pin_root_base: Cell::new(false),
            removal_noop: Cell::new(false),
            floated: Default::default(),
        })
    }
}
impl FactoryBaseImpl for MutationFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn pin_dockable_core(&self, item: Option<Rc<dyn IDockable>>) {
        self.trace
            .borrow_mut()
            .push(format!("pin:{}", label(&item)));
    }
    fn pin_dockable_core_with_root(
        &self,
        item: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
        unpin: Option<Rc<dyn IToolDock>>,
    ) {
        self.trace
            .borrow_mut()
            .push(format!("pin-root:{}", label(&item)));
        if self.pin_root_base.get() {
            self.component
                .base()
                .pin_dockable_core_with_root(item, root, unpin);
        }
    }
}
impl FactoryImpl for MutationFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.trace
            .borrow_mut()
            .push(format!("init:{}:{}", label(&item), label(&owner)));
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        if item.is_none() {
            this.trace.borrow_mut().push("root:null".into());
            this.null_root.borrow().clone()
        } else {
            this.component.base().find_root(item, predicate)
        }
    }
    fn remove_dockable(this: &Self, item: Option<Rc<dyn IDockable>>, collapse: bool) {
        if this.removal_noop.get() {
            this.trace
                .borrow_mut()
                .push(format!("remove:{}", label(&item)));
        } else {
            this.component.base().remove_dockable(item, collapse);
        }
    }
    fn create_window_from_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        this.trace.borrow_mut().push("window".into());
        this.floated.replace(item.and_then(|item| item.as_dock()));
        None
    }
}
impl_factory!(MutationFactory);
fn failure(action: impl FnOnce()) -> String {
    let value = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    value
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| value.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn negative_insert_does_not_access_null_dock_or_item_or_dispatch_initialization() {
    let factory = MutationFactory::new();
    factory.insert_dockable(None, None, -1);
    let dock = DocumentDock::new();
    factory.insert_dockable(Some(dock.clone()), None, -1);
    assert!(factory.trace.borrow().is_empty());
    assert!(dock.visible_dockables().is_none());
    Factory::new().insert_dockable(None, None, -1);
}
#[test]
fn null_insert_keeps_original_list_and_orders_init_mutation_empty_count_and_added() {
    let factory = MutationFactory::new();
    let dock = DocumentDock::new();
    dock.set_id("docs".into());
    let list = factory.create_list::<Option<Rc<dyn IDockable>>>(vec![]);
    dock.set_visible_dockables(Some(list.clone()));
    let trace = factory.trace.clone();
    let token = list
        .add_collection_changed(Rc::new(move |args| {
            assert_eq!(args.new_items.len(), 1);
            assert!(args.new_items[0].is_none());
            assert_eq!(args.new_starting_index, 0);
            trace.borrow_mut().push("list".into());
        }))
        .unwrap();
    let trace = factory.trace.clone();
    let event = factory.dockable_added(Rc::new(move |args| {
        assert!(args.dockable().is_none());
        trace.borrow_mut().push("added:null".into());
    }));
    factory.insert_dockable(Some(dock.clone()), None, 0);
    assert_eq!(
        &*factory.trace.borrow(),
        &["init:null:docs", "list", "added:null"]
    );
    assert!(list.ptr_eq(&dock.visible_dockables().unwrap()));
    assert_eq!(list.count(), 1);
    assert!(list.get(0).is_none());
    assert!(dock.is_empty());
    assert_eq!(dock.opened_dockables_count(), 1);
    event.dispose();
    list.remove_collection_changed(token);
}
#[test]
fn insert_null_target_and_invalid_index_fail_after_original_init_and_list_creation_boundaries() {
    let factory = MutationFactory::new();
    let tool = Tool::new();
    tool.set_id("tool".into());
    assert!(failure(|| factory.insert_dockable(None, Some(tool), 0))
        .contains("insert dock.VisibleDockables"));
    assert_eq!(&*factory.trace.borrow(), &["init:tool:null"]);
    factory.trace.borrow_mut().clear();
    let dock = DocumentDock::new();
    dock.set_id("docs".into());
    failure(|| factory.insert_dockable(Some(dock.clone()), None, 2));
    assert_eq!(&*factory.trace.borrow(), &["init:null:docs"]);
    assert_eq!(dock.visible_dockables().unwrap().count(), 0);
    let untouched = DocumentDock::new();
    failure(|| Factory::new().insert_dockable(Some(untouched.clone()), None, 0));
    assert!(untouched.visible_dockables().is_none());
}
#[test]
fn removal_forwards_null_through_root_and_pin_cleanup_before_original_owner_failure() {
    let factory = MutationFactory::new();
    assert!(failure(|| factory.remove_dockable(None, true)).contains("removal dockable.Owner"));
    assert_eq!(&*factory.trace.borrow(), &["root:null"]);
    factory.trace.borrow_mut().clear();
    let root = RootDock::new();
    let pins = factory.create_list(vec![None]);
    root.set_left_pinned_dockables(Some(pins.clone()));
    factory.null_root.replace(Some(root));
    assert!(failure(|| factory.remove_dockable(None, true)).contains("removal dockable.Owner"));
    assert_eq!(&*factory.trace.borrow(), &["root:null", "pin:null"]);
    assert_eq!(pins.count(), 1);
    assert!(pins.get(0).is_none());
    assert!(failure(|| Factory::new().remove_dockable(None, true)).contains("dockable.Owner"));
}
#[test]
fn protected_pin_cleanup_passes_null_to_explicit_root_override_before_default_owner_failure() {
    let factory = MutationFactory::new();
    factory.component.base().pin_dockable_core(None);
    assert_eq!(&*factory.trace.borrow(), &["root:null"]);
    factory.trace.borrow_mut().clear();
    factory.null_root.replace(Some(RootDock::new()));
    factory.component.base().pin_dockable_core(None);
    assert_eq!(&*factory.trace.borrow(), &["root:null", "pin-root:null"]);
    factory.trace.borrow_mut().clear();
    factory.pin_root_base.set(true);
    assert!(
        failure(|| factory.component.base().pin_dockable_core(None)).contains("pin dockable.Owner")
    );
    assert_eq!(&*factory.trace.borrow(), &["root:null", "pin-root:null"]);
}
#[test]
fn whole_pane_float_passes_null_snapshot_entries_through_original_remove_add_and_moved_slots() {
    let factory = MutationFactory::new();
    factory.removal_noop.set(true);
    let root = RootDock::new();
    let dock = DocumentDock::new();
    dock.set_id("docs".into());
    dock.set_owner(Some(root.clone()));
    dock.set_is_collapsable(false);
    let item = Tool::new();
    item.set_id("trigger".into());
    item.set_owner(Some(dock.clone()));
    let original = factory.create_list(vec![None, Some(item.clone() as Rc<dyn IDockable>)]);
    dock.set_visible_dockables(Some(original.clone()));
    let trace = factory.trace.clone();
    let subscription = factory.dockable_moved(Rc::new(move |args| {
        trace
            .borrow_mut()
            .push(format!("moved:{}", label(&args.dockable())))
    }));
    factory.float_all_dockables(Some(item.clone()));
    assert_eq!(
        &*factory.trace.borrow(),
        &[
            "remove:null",
            "init:null:docs",
            "moved:null",
            "remove:trigger",
            "init:trigger:docs",
            "moved:trigger",
            "window"
        ]
    );
    let target = factory.floated.borrow().clone().unwrap();
    let list = target.visible_dockables().unwrap();
    assert_eq!(list.count(), 2);
    assert!(list.get(0).is_none());
    assert!(Rc::ptr_eq(
        &list.get(1).unwrap(),
        &(item as Rc<dyn IDockable>)
    ));
    assert_eq!(original.count(), 2);
    assert!(original.get(0).is_none());
    subscription.dispose();
}
