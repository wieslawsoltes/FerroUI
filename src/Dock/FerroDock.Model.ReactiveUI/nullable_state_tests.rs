//! Source nullable state hooks and root recursion through the actual factory.
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};

struct StateFactory {
    component: Factory,
    trace: RefCell<Vec<String>>,
    base_resolve: Cell<bool>,
    intercept_null_root: Cell<bool>,
    null_root: RefCell<Option<Rc<dyn IRootDock>>>,
}
impl StateFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: RefCell::new(Vec::new()),
            base_resolve: Cell::new(false),
            intercept_null_root: Cell::new(false),
            null_root: RefCell::new(None),
        })
    }
    fn record(&self, name: &str, item: &Option<Rc<dyn IDockable>>) {
        self.trace.borrow_mut().push(format!(
            "{name}:{}",
            item.as_ref()
                .map_or_else(|| "null".into(), |item| item.id())
        ));
    }
}
impl FactoryBaseImpl for StateFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn update_docking_window_state(&self, item: Option<Rc<dyn IDockable>>) {
        self.record("update", &item);
    }
    fn update_docking_window_state_recursive(&self, item: Option<Rc<dyn IDockable>>) {
        self.record("recurse", &item);
        self.component
            .base()
            .update_docking_window_state_recursive(item);
    }
    fn resolve_docking_window_state(&self, item: Option<Rc<dyn IDockable>>) -> DockingWindowState {
        self.record("resolve", &item);
        if self.base_resolve.get() {
            self.component.base().resolve_docking_window_state(item)
        } else {
            DockingWindowState::DOCKED
        }
    }
}
impl FactoryImpl for StateFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        this.record("root", &item);
        if item.is_none() && this.intercept_null_root.get() {
            this.null_root.borrow().clone()
        } else {
            this.component.base().find_root(item, predicate)
        }
    }
}
impl_factory!(StateFactory);
fn failure(action: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(action)).expect_err("original null dereference");
    error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn recursive_state_hooks_receive_null_children_before_split_extras_without_duplicate_content() {
    let factory = StateFactory::new();
    let root = SplitViewDock::new();
    root.set_id("split".into());
    let child = Document::new();
    child.set_id("child".into());
    let pane = Tool::new();
    pane.set_id("pane".into());
    root.set_visible_dockables(Some(
        factory.create_list(vec![None, Some(child as Rc<dyn IDockable>)]),
    ));
    root.set_pane_dockable(Some(pane.clone()));
    root.set_content_dockable(Some(pane));
    factory
        .component
        .base()
        .update_docking_window_state_recursive(Some(root));
    assert_eq!(
        &*factory.trace.borrow(),
        &[
            "update:split",
            "recurse:null",
            "update:null",
            "recurse:child",
            "update:child",
            "recurse:pane",
            "update:pane"
        ]
    );
}
#[test]
fn base_null_update_calls_resolver_before_assignment_and_restores_sync_suppression_on_failure() {
    let factory = StateFactory::new();
    assert!(
        failure(|| factory.component.base().update_docking_window_state(None))
            .contains("DockingState assignment")
    );
    assert_eq!(&*factory.trace.borrow(), &["resolve:null"]);
    factory.trace.borrow_mut().clear();
    let tool = Tool::new();
    tool.set_id("tool".into());
    factory
        .component
        .base()
        .on_docking_window_state_property_changed(tool, DockingWindowStateProperty::IsSelected);
    assert_eq!(
        factory.trace.borrow().first().map(String::as_str),
        Some("resolve:tool")
    );
}
#[test]
fn find_root_forwards_null_after_owner_changes_to_the_original_virtual_override() {
    let factory = StateFactory::new();
    factory.intercept_null_root.set(true);
    let fallback = RootDock::new();
    let expected: Rc<dyn IRootDock> = fallback.clone();
    factory.null_root.replace(Some(fallback));
    assert!(Rc::ptr_eq(
        &factory.find_root(None, None).unwrap(),
        &expected
    ));
    let first = RootDock::new();
    let leaf = Tool::new();
    leaf.set_id("leaf".into());
    leaf.set_owner(Some(first.clone()));
    factory.trace.borrow_mut().clear();
    let changed = leaf.clone();
    let predicate: RootPredicate = Rc::new(move |_| {
        changed.set_owner(None);
        false
    });
    assert!(Rc::ptr_eq(
        &factory.find_root(Some(leaf), Some(predicate)).unwrap(),
        &expected
    ));
    assert_eq!(&*factory.trace.borrow(), &["root:leaf", "root:null"]);
    assert!(failure(|| {
        Factory::new().find_root(None, None);
    })
    .contains("dockable.Owner"));
}
#[test]
fn null_resolver_preserves_root_lookup_and_original_owner_failure_order_and_null_pin_membership() {
    let factory = StateFactory::new();
    assert!(failure(|| {
        factory.component.base().resolve_docking_window_state(None);
    })
    .contains("dockable.Owner"));
    assert_eq!(&*factory.trace.borrow(), &["root:null"]);
    factory.intercept_null_root.set(true);
    factory.trace.borrow_mut().clear();
    assert!(failure(|| {
        factory.component.base().resolve_docking_window_state(None);
    })
    .contains("document-area dockable.OriginalOwner"));
    let root = RootDock::new();
    root.set_left_pinned_dockables(Some(factory.create_list(vec![None])));
    factory.null_root.replace(Some(root.clone()));
    assert!(factory.is_dockable_pinned(None, Some(root.clone())));
    assert!(factory.is_dockable_pinned(None, None));
    assert!(failure(|| {
        factory.component.base().resolve_docking_window_state(None);
    })
    .contains("floating-area dockable.OriginalOwner"));
    root.set_left_pinned_dockables(None);
    assert!(!factory.is_dockable_pinned(None, Some(root)));
}
