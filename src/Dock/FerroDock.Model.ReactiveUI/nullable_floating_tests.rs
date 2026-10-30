//! Original nullable floating/window overloads, preparation and source identity.
use crate::{controls::*, core::DockWindow, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct WindowFactory {
    component: Factory,
    trace: Rc<RefCell<Vec<String>>>,
    root: RefCell<Option<Rc<dyn IRootDock>>>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    prepare_base: Cell<bool>,
    abort_prepare: Cell<bool>,
    mutate_modal: Cell<bool>,
    last_options: RefCell<Option<Rc<DockWindowOptions>>>,
    last_item: RefCell<Option<Rc<dyn IDockable>>>,
    last_dock: RefCell<Option<Rc<dyn IDock>>>,
    last_window: RefCell<Option<Rc<dyn IDockWindow>>>,
    coordinates: Cell<(f64, f64, f64, f64)>,
    collapse: RefCell<Vec<bool>>,
}
fn name(item: &Option<Rc<dyn IDockable>>) -> String {
    item.as_ref().map_or_else(|| "null".into(), |i| i.id())
}
impl WindowFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: Default::default(),
            root: Default::default(),
            window: Default::default(),
            prepare_base: Cell::new(false),
            abort_prepare: Cell::new(false),
            mutate_modal: Cell::new(false),
            last_options: Default::default(),
            last_item: Default::default(),
            last_dock: Default::default(),
            last_window: Default::default(),
            coordinates: Cell::new((0.0, 0.0, 0.0, 0.0)),
            collapse: Default::default(),
        })
    }
}
impl FactoryBaseImpl for WindowFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn prepare_window_options_for_dockable(
        &self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.trace
            .borrow_mut()
            .push(format!("prepare:{}", name(&item)));
        self.last_options.replace(options.clone());
        assert!(!self.abort_prepare.get(), "prepare aborted");
        if self.mutate_modal.get() {
            options.as_ref().unwrap().set_is_modal(true);
        }
        if self.prepare_base.get() {
            self.component
                .base()
                .prepare_window_options_for_dockable(item, options);
        }
    }
    fn apply_window_options(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.trace.borrow_mut().push("apply".into());
        self.last_options.replace(options.clone());
        self.last_window.replace(window.clone());
        self.component.base().apply_window_options(window, options);
    }
}
impl FactoryImpl for WindowFactory {
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
            .push(format!("find:{}", name(&item)));
        this.root.borrow().clone()
    }
    fn create_window_from(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        this.trace
            .borrow_mut()
            .push(format!("plain:{}", name(&item)));
        this.last_item.replace(item);
        this.window.borrow().clone()
    }
    fn create_window_from_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        this.trace
            .borrow_mut()
            .push(format!("create-options:{}", name(&item)));
        this.last_item.replace(item);
        this.last_options.replace(options);
        this.window.borrow().clone()
    }
    fn float_dockable_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        this.trace
            .borrow_mut()
            .push(format!("float:{}", name(&item)));
        this.last_item.replace(item);
        this.last_options.replace(options);
    }
    fn float_all_dockables_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        this.trace.borrow_mut().push(format!("all:{}", name(&item)));
        this.last_item.replace(item);
        this.last_options.replace(options);
    }
    #[allow(clippy::too_many_arguments)]
    fn split_to_window_with_options(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        this.trace.borrow_mut().push(format!(
            "split:{}:{}",
            name(&dock.clone().map(|d| d as Rc<dyn IDockable>)),
            name(&item)
        ));
        this.last_dock.replace(dock);
        this.last_item.replace(item);
        this.last_options.replace(options);
        this.coordinates.set((x, y, width, height));
    }
    fn remove_dockable(this: &Self, item: Option<Rc<dyn IDockable>>, collapse: bool) {
        this.trace
            .borrow_mut()
            .push(format!("remove:{}", name(&item)));
        this.collapse.borrow_mut().push(collapse);
    }
}
impl_factory!(WindowFactory);
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn default_null_creation_preserves_options_ownership_short_circuits() {
    let f = Factory::new();
    assert!(f.create_window_from(None).is_none());
    assert!(f.create_window_from_with_options(None, None).is_none());
    let options = DockWindowOptions::new();
    assert!(f
        .create_window_from_with_options(None, Some(options.clone()))
        .is_none());
    assert!(options.parent_window().is_none());
    options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
    assert!(failure(|| {
        f.create_window_from_with_options(None, Some(options.clone()));
    })
    .contains("dockable.Owner"));
    let parent = DockWindow::new();
    options.set_parent_window(Some(parent.clone()));
    assert!(f
        .create_window_from_with_options(None, Some(options.clone()))
        .is_none());
    assert!(Rc::ptr_eq(
        &options.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
}
#[test]
fn plain_null_and_unsupported_creation_do_not_query_root_or_eagerly_create_a_window() {
    let f = WindowFactory::new();
    f.window.replace(Some(DockWindow::new()));
    f.root.replace(Some(RootDock::new()));
    assert!(f.component.base().create_window_from(None).is_none());
    assert!(f
        .component
        .base()
        .create_window_from(Some(StackDock::new()))
        .is_none());
    assert!(f.trace.borrow().is_empty());
}
#[test]
fn float_and_split_wrappers_forward_actual_null_to_enclosing_options_overloads() {
    let f = WindowFactory::new();
    f.float_dockable(None);
    f.float_all_dockables(None);
    f.split_to_window(None, None, 1.0, 2.0, 300.0, 400.0);
    assert_eq!(
        &*f.trace.borrow(),
        &["float:null", "all:null", "split:null:null"]
    );
    assert!(f.last_item.borrow().is_none());
    assert!(f.last_dock.borrow().is_none());
    assert!(f.last_options.borrow().is_none());
    assert_eq!(f.coordinates.get(), (1.0, 2.0, 300.0, 400.0));
}
#[test]
fn default_float_null_fails_at_operation_context_while_custom_split_root_none_is_noop() {
    let f = Factory::new();
    assert!(failure(|| f.float_dockable(None)).contains("ResolveOperationDock dockable.Owner"));
    assert!(failure(|| f.float_all_dockables(None)).contains("ResolveOperationDock dockable.Owner"));
    assert!(
        failure(|| f.split_to_window(None, None, 0.0, 0.0, 1.0, 1.0)).contains("dockable.Owner")
    );
    let probe = WindowFactory::new();
    probe
        .component
        .base()
        .split_to_window_with_options(None, None, 0.0, 0.0, 1.0, 1.0, None);
    assert_eq!(&*probe.trace.borrow(), &["find:null"]);
}
#[test]
fn ownership_preparation_of_null_mutates_same_options_even_when_creation_returns_none() {
    let f = WindowFactory::new();
    f.prepare_base.set(true);
    let root = RootDock::new();
    let parent = DockWindow::new();
    root.set_window(Some(parent.clone()));
    f.root.replace(Some(root));
    let options = DockWindowOptions::new();
    options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
    assert!(f
        .component
        .base()
        .create_window_from_with_options(None, Some(options.clone()))
        .is_none());
    assert_eq!(
        &*f.trace.borrow(),
        &["prepare:null", "find:null", "plain:null"]
    );
    assert!(Rc::ptr_eq(
        &options.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
    assert!(Rc::ptr_eq(
        f.last_options.borrow().as_ref().unwrap(),
        &options
    ));
    assert!(f.last_window.borrow().is_none());
}
#[test]
fn actual_prepare_override_can_abort_or_mutate_options_before_plain_result_is_applied() {
    let f = WindowFactory::new();
    let window = DockWindow::new();
    f.window.replace(Some(window.clone()));
    let options = DockWindowOptions::new();
    f.abort_prepare.set(true);
    assert!(failure(|| {
        f.component
            .base()
            .create_window_from_with_options(None, Some(options.clone()));
    })
    .contains("prepare aborted"));
    assert_eq!(&*f.trace.borrow(), &["prepare:null"]);
    assert!(!window.is_modal());
    f.trace.borrow_mut().clear();
    f.abort_prepare.set(false);
    f.mutate_modal.set(true);
    let result = f
        .component
        .base()
        .create_window_from_with_options(None, Some(options.clone()))
        .unwrap();
    assert!(Rc::ptr_eq(
        &result,
        &(window.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(window.is_modal());
    assert!(Rc::ptr_eq(
        f.last_options.borrow().as_ref().unwrap(),
        &options
    ));
    assert!(Rc::ptr_eq(
        f.last_window.borrow().as_ref().unwrap(),
        &result
    ));
    assert_eq!(&*f.trace.borrow(), &["prepare:null", "plain:null", "apply"]);
    f.trace.borrow_mut().clear();
    f.mutate_modal.set(false);
    let next = f
        .component
        .base()
        .create_window_from_with_options(None, None)
        .unwrap();
    assert!(Rc::ptr_eq(&next, &result));
    assert!(window.is_modal());
    assert!(f.last_options.borrow().is_none());
    assert_eq!(&*f.trace.borrow(), &["prepare:null", "plain:null", "apply"]);
}
#[test]
fn split_null_reaches_original_prepare_remove_event_and_creation_in_order() {
    let f = WindowFactory::new();
    f.root.replace(Some(RootDock::new()));
    let options = DockWindowOptions::new();
    let trace = f.trace.clone();
    let event = f.dockable_undocked(Rc::new(move |args| {
        assert!(args.dockable().is_none());
        assert_eq!(args.operation(), DockOperation::Window);
        trace.borrow_mut().push("undocked:null".into());
    }));
    f.component.base().split_to_window_with_options(
        None,
        None,
        1.0,
        2.0,
        300.0,
        400.0,
        Some(options.clone()),
    );
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "find:null",
            "prepare:null",
            "remove:null",
            "undocked:null",
            "create-options:null"
        ]
    );
    assert_eq!(&*f.collapse.borrow(), &[true]);
    assert!(Rc::ptr_eq(
        f.last_options.borrow().as_ref().unwrap(),
        &options
    ));
    event.dispose();
}
#[test]
fn ordinary_float_keeps_original_coordinate_fallback_and_nullable_overload_handle_identity() {
    let f = WindowFactory::new();
    let owner = ToolDock::new();
    owner.set_id("owner".into());
    owner.set_pointer_screen_position(f64::NAN, 200.0);
    owner.set_visible_bounds(30.0, 40.0, 640.0, 480.0);
    let item = Tool::new();
    item.set_id("item".into());
    item.set_owner(Some(owner.clone()));
    item.set_visible_bounds(11.0, 12.0, 8.0, 80.0);
    let options = DockWindowOptions::new();
    f.component
        .base()
        .float_dockable_with_options(Some(item.clone()), Some(options.clone()));
    assert_eq!(f.coordinates.get(), (11.0, 200.0, 640.0, 80.0));
    assert!(Rc::ptr_eq(
        f.last_item.borrow().as_ref().unwrap(),
        &(item as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        f.last_dock.borrow().as_ref().unwrap(),
        &(owner as Rc<dyn IDock>)
    ));
    assert!(Rc::ptr_eq(
        f.last_options.borrow().as_ref().unwrap(),
        &options
    ));
    assert_eq!(
        &*f.trace.borrow(),
        &["find:item", "prepare:item", "split:owner:item"]
    );
}
