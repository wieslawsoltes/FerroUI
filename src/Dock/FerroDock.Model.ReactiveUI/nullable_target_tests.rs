//! Original nullable target, pin-root and protected selection/helper contracts.
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    controls::{IRootDock, IToolDock},
    core::*,
    FactoryBase, FactoryBaseImpl,
};
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct TargetFactory {
    component: Factory,
    trace: RefCell<Vec<String>>,
    query_root: RefCell<Option<Rc<dyn IRootDock>>>,
    selection: Rc<dyn IDockable>,
}
fn name(item: &Option<Rc<dyn IDockable>>) -> String {
    item.as_ref()
        .map_or_else(|| "null".into(), |item| item.id())
}
impl TargetFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: Default::default(),
            query_root: Default::default(),
            selection: Document::new(),
        })
    }
}
impl FactoryBaseImpl for TargetFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn get_active_dockable_after_removal(
        &self,
        dock: Option<Rc<dyn IDock>>,
        removed: Option<Rc<dyn IDockable>>,
        index: i32,
    ) -> Option<Rc<dyn IDockable>> {
        self.trace.borrow_mut().push(format!(
            "selection:{}:{}:{index}",
            name(&dock.map(|d| d as Rc<dyn IDockable>)),
            name(&removed)
        ));
        Some(self.selection.clone())
    }
    fn pin_dockable_core_with_root(
        &self,
        item: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
        _: Option<Rc<dyn IToolDock>>,
    ) {
        self.trace.borrow_mut().push(format!(
            "pin:{}:{}",
            name(&item),
            name(&root.map(|r| r as Rc<dyn IDockable>))
        ));
    }
}
impl FactoryImpl for TargetFactory {
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
            .push(format!("init:{}:{}", name(&item), name(&owner)));
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        this.trace
            .borrow_mut()
            .push(format!("root:{}", name(&item)));
        this.query_root.borrow().clone()
    }
}
impl_factory!(TargetFactory);
fn failure(action: impl FnOnce()) -> String {
    let value = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    value
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| value.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn add_null_target_runs_actual_init_override_before_first_visible_list_access() {
    let factory = TargetFactory::new();
    let item = Tool::new();
    item.set_id("item".into());
    assert!(
        failure(|| factory.add_dockable(None, Some(item))).contains("add dock.VisibleDockables")
    );
    assert_eq!(&*factory.trace.borrow(), &["init:item:null"]);
    factory.trace.borrow_mut().clear();
    assert!(failure(|| factory.add_dockable(None, None)).contains("add dock.VisibleDockables"));
    assert_eq!(&*factory.trace.borrow(), &["init:null:null"]);
    let factory = Factory::new();
    let item = Tool::new();
    item.set_owner(Some(DocumentDock::new()));
    assert!(failure(|| factory.add_dockable(None, Some(item.clone())))
        .contains("add dock.VisibleDockables"));
    assert!(item.owner().is_none());
    assert!(Rc::ptr_eq(
        &item.factory().unwrap(),
        &(factory.clone() as Rc<dyn IFactory>)
    ));
    assert!(failure(|| factory.add_dockable(None, None)).contains("dockable.Context"));
}
#[test]
fn protected_visible_helpers_access_null_target_before_index_or_item_processing() {
    let factory = Factory::new();
    let base = factory.base();
    assert!(failure(|| base.add_visible_dockable(None, None))
        .contains("add_visible_dockable dock.VisibleDockables"));
    assert!(failure(|| base.insert_visible_dockable(None, -1, None))
        .contains("insert_visible_dockable dock.VisibleDockables"));
    assert!(failure(|| base.remove_visible_dockable(None, None))
        .contains("remove_visible_dockable dock.VisibleDockables"));
    assert!(failure(|| base.remove_all_visible_dockables(None))
        .contains("remove_all_visible_dockables dock.VisibleDockables"));
    assert!(failure(|| base.remove_visible_dockable_at(None, -1))
        .contains("remove_visible_dockable_at dock.VisibleDockables"));
    assert!(failure(|| base.cleanup_orphaned_splitters(None))
        .contains("cleanup_orphaned_splitters dock.VisibleDockables"));
    assert!(failure(|| {
        base.get_active_dockable_after_removal(None, None, -1);
    })
    .contains("selection dock.VisibleDockables"));
}
#[test]
fn source_selection_ignores_null_removed_item_and_retains_actual_child_or_splitter_result() {
    let factory = Factory::new();
    let dock = DocumentDock::new();
    let base = factory.base();
    assert!(base
        .get_active_dockable_after_removal(Some(dock.clone()), None, -1)
        .is_none());
    let child: Rc<dyn IDockable> = Document::new();
    let splitter: Rc<dyn IDockable> = ProportionalDockSplitter::new();
    let list = factory.create_list(vec![None, Some(child.clone()), Some(splitter)]);
    dock.set_visible_dockables(Some(list.clone()));
    assert!(base
        .get_active_dockable_after_removal(Some(dock.clone()), None, 1)
        .is_none());
    let selected = base
        .get_active_dockable_after_removal(Some(dock.clone()), None, 2)
        .unwrap();
    assert!(Rc::ptr_eq(&selected, &child));
    assert!(base
        .get_active_dockable_after_removal(Some(dock.clone()), None, 3)
        .is_none());
    failure(|| {
        base.get_active_dockable_after_removal(Some(dock.clone()), None, 5);
    });
    assert!(list.ptr_eq(&dock.visible_dockables().unwrap()));
    assert_eq!(list.count(), 3);
}
#[test]
fn protected_overrides_receive_null_target_removed_item_and_explicit_pin_root() {
    let factory = TargetFactory::new();
    let selected =
        FactoryBaseImpl::get_active_dockable_after_removal(factory.as_ref(), None, None, 2)
            .unwrap();
    assert!(Rc::ptr_eq(&selected, &factory.selection));
    FactoryBaseImpl::pin_dockable_core_with_root(factory.as_ref(), None, None, None);
    assert_eq!(
        &*factory.trace.borrow(),
        &["selection:null:null:2", "pin:null:null"]
    );
}
#[test]
fn null_pin_root_can_return_without_root_dereference_and_null_item_fails_first() {
    let factory = TargetFactory::new();
    let item = Tool::new();
    item.set_id("item".into());
    factory
        .component
        .base()
        .pin_dockable_core_with_root(Some(item.clone()), None, None);
    assert_eq!(&*factory.trace.borrow(), &["root:item"]);
    factory.trace.borrow_mut().clear();
    let owner = ToolDock::new();
    item.set_owner(Some(owner));
    factory
        .component
        .base()
        .pin_dockable_core_with_root(Some(item), None, None);
    assert_eq!(&*factory.trace.borrow(), &["root:item"]);
    factory.trace.borrow_mut().clear();
    assert!(failure(|| factory
        .component
        .base()
        .pin_dockable_core_with_root(None, None, None))
    .contains("pin dockable.Owner"));
    assert!(factory.trace.borrow().is_empty());
}
#[test]
fn visible_pin_updates_bounds_before_null_root_collection_failure_and_keeps_source_list() {
    for item_bounds in [true, false] {
        let factory = TargetFactory::new();
        let item = Tool::new();
        item.set_id("item".into());
        let owner = ToolDock::new();
        item.set_owner(Some(owner.clone()));
        owner.set_visible_bounds(10.0, 20.0, 210.0, 160.0);
        if item_bounds {
            item.set_visible_bounds(40.0, 50.0, 310.0, 260.0);
        }
        let list = factory.create_list(vec![Some(item.clone() as Rc<dyn IDockable>)]);
        owner.set_visible_dockables(Some(list.clone()));
        assert!(
            failure(|| factory.component.base().pin_dockable_core_with_root(
                Some(item.clone()),
                None,
                None
            ))
            .contains("root pinned collection")
        );
        let expected = if item_bounds {
            (0.0, 0.0, 310.0, 260.0)
        } else {
            (0.0, 0.0, 210.0, 160.0)
        };
        assert_eq!(item.get_pinned_bounds(), expected);
        assert_eq!(&*factory.trace.borrow(), &["root:item"]);
        assert!(list.ptr_eq(&owner.visible_dockables().unwrap()));
        assert_eq!(list.count(), 1);
        assert!(Rc::ptr_eq(
            &list.get(0).unwrap(),
            &(item as Rc<dyn IDockable>)
        ));
    }
}
#[test]
fn pin_membership_fallback_keeps_explicit_null_root_for_original_unpin_failure() {
    for tool_owner in [true, false] {
        let factory = TargetFactory::new();
        let item = Tool::new();
        item.set_id("item".into());
        let owner = ToolDock::new();
        if tool_owner {
            item.set_owner(Some(owner.clone()));
        }
        let list = factory.create_list(vec![Some(item.clone() as Rc<dyn IDockable>)]);
        owner.set_visible_dockables(Some(list.clone()));
        let root = RootDock::new();
        let pins = factory.create_list(vec![Some(item.clone() as Rc<dyn IDockable>)]);
        root.set_left_pinned_dockables(Some(pins.clone()));
        factory.query_root.replace(Some(root));
        assert!(
            failure(|| factory.component.base().pin_dockable_core_with_root(
                Some(item.clone()),
                None,
                None
            ))
            .contains("unpin root.PinnedDock")
        );
        assert_eq!(&*factory.trace.borrow(), &["root:item"]);
        assert_eq!(pins.count(), 1);
        assert_eq!(list.count(), 1);
        let (_, _, width, height) = item.get_pinned_bounds();
        assert!(width.is_nan() && height.is_nan());
    }
}
