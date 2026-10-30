//! Original nullable window collections, virtual lifecycle slots and traversal failure order.
use crate::test_host::RecordingHost;
use crate::test_support::TestWindow;
use crate::{controls::*, core::DockWindow, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    collections::ListHandle, controls::IRootDock, core::*, DockState, FactoryBase, FactoryBaseImpl,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
type Action = Rc<dyn Fn()>;
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("original failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
fn window_name(window: &Option<Rc<dyn IDockWindow>>) -> String {
    window.as_ref().map_or_else(|| "null".into(), |w| w.id())
}
struct WindowsFactory {
    component: Factory,
    trace: RefCell<Vec<String>>,
    init_base: Cell<bool>,
    explicit_base: Cell<bool>,
    host_enabled: Cell<bool>,
    host: RefCell<Option<Rc<dyn IHostWindow>>>,
    windows: RefCell<Vec<Option<Rc<dyn IDockWindow>>>>,
    owners: RefCell<Vec<Option<Rc<dyn IDockable>>>>,
    init_items: RefCell<Vec<Option<Rc<dyn IDockable>>>>,
    action: RefCell<Option<Action>>,
}
impl WindowsFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: Default::default(),
            init_base: Cell::new(false),
            explicit_base: Cell::new(false),
            host_enabled: Cell::new(false),
            host: Default::default(),
            windows: Default::default(),
            owners: Default::default(),
            init_items: Default::default(),
            action: Default::default(),
        })
    }
}
impl FactoryBaseImpl for WindowsFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl IDockWindowHostFactory for WindowsFactory {
    fn create_host_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
    ) -> Option<Rc<dyn IHostWindow>> {
        self.trace
            .borrow_mut()
            .push(format!("host:{}", window_name(&window)));
        self.windows.borrow_mut().push(window);
        self.host.borrow().clone()
    }
}
impl FactoryImpl for WindowsFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn as_dock_window_host_factory(this: Rc<Self>) -> Option<Rc<dyn IDockWindowHostFactory>> {
        if this.host_enabled.get() {
            Some(this)
        } else {
            None
        }
    }
    fn on_window_added(this: &Self, window: Option<Rc<dyn IDockWindow>>) {
        this.trace
            .borrow_mut()
            .push(format!("added:{}", window_name(&window)));
        this.component.base().on_window_added(window);
        let action = this.action.borrow().clone();
        if let Some(action) = action {
            action();
        }
    }
    fn init_dock_window(
        this: &Self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.trace
            .borrow_mut()
            .push(format!("init:{}", window_name(&window)));
        this.windows.borrow_mut().push(window.clone());
        this.owners.borrow_mut().push(owner.clone());
        if this.init_base.get() {
            this.component.base().init_dock_window(window, owner);
        }
    }
    fn init_dock_window_with_host(
        this: &Self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
        host: Option<Rc<dyn IHostWindow>>,
    ) {
        this.trace
            .borrow_mut()
            .push(format!("explicit:{}", window_name(&window)));
        this.windows.borrow_mut().push(window.clone());
        this.owners.borrow_mut().push(owner.clone());
        if this.explicit_base.get() {
            this.component
                .base()
                .init_dock_window_with_host(window, owner, host);
        }
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.trace.borrow_mut().push(format!(
            "dock-init:{}",
            item.as_ref().map_or_else(|| "null".into(), |i| i.id())
        ));
        this.init_items.borrow_mut().push(item);
        this.owners.borrow_mut().push(owner);
    }
}
impl_factory!(WindowsFactory);
#[test]
fn null_defaults_keep_original_first_access_and_negative_insert_guard() {
    let f = Factory::new();
    f.insert_window(None, None, -1);
    for (label, error) in [
        ("add", failure(|| f.add_window(None, None))),
        ("insert", failure(|| f.insert_window(None, None, 0))),
        ("remove", failure(|| f.remove_window(None))),
        ("close", failure(|| f.close_window(None))),
        ("init", failure(|| f.init_dock_window(None, None))),
        (
            "explicit",
            failure(|| f.init_dock_window_with_host(None, None, None)),
        ),
        (
            "resolve",
            failure(|| {
                f.base().resolve_window_collection_root(None);
            }),
        ),
    ] {
        let expected = match label {
            "add" | "insert" => "root.Windows",
            "remove" => "window.Owner",
            "close" => "window.Layout",
            "init" => "window.Id",
            "explicit" => "window.Host",
            _ => "root.Window",
        };
        assert!(error.contains(expected), "{label}: {error}");
    }
}
#[test]
fn add_and_insert_keep_null_slots_payloads_and_initialization_owner_identity() {
    let f = WindowsFactory::new();
    let root = RootDock::new();
    let list: ListHandle<Option<Rc<dyn IDockWindow>>> = ListHandle::new();
    root.set_windows(Some(list.clone()));
    let payloads = Rc::new(RefCell::new(Vec::new()));
    let changes = payloads.clone();
    list.add_collection_changed(Rc::new(move |e| {
        changes
            .borrow_mut()
            .push((e.new_starting_index, e.new_items.to_vec()))
    }));
    f.add_window(Some(root.clone()), None);
    f.insert_window(Some(root.clone()), None, 0);
    assert_eq!(
        &*f.trace.borrow(),
        &["added:null", "init:null", "added:null", "init:null"]
    );
    assert_eq!(list.count(), 2);
    assert!(list.iter().all(|w| w.is_none()));
    assert!(root.windows().unwrap().ptr_eq(&list));
    assert_eq!(payloads.borrow().len(), 2);
    for (index, (actual, items)) in payloads.borrow().iter().enumerate() {
        assert_eq!(*actual, 0);
        assert_eq!(items.len(), 1);
        assert!(items[0].is_none());
        assert!(f.windows.borrow()[index].is_none());
    }
    for owner in f.owners.borrow().iter() {
        assert!(Rc::ptr_eq(
            owner.as_ref().unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
    }
}
#[test]
fn default_initialization_fails_after_null_is_committed_and_added_event_is_raised() {
    let f = Factory::new();
    let root = RootDock::new();
    let events = Rc::new(Cell::new(0));
    let count = events.clone();
    let weak = Rc::downgrade(&root);
    f.window_added(Rc::new(move |e| {
        assert!(e.window().is_none());
        assert!(weak.upgrade().unwrap().windows().unwrap().get(0).is_none());
        count.set(count.get() + 1);
    }));
    assert!(failure(|| f.add_window(Some(root.clone()), None)).contains("window.Id"));
    assert_eq!(events.get(), 1);
    assert_eq!(root.windows().unwrap().count(), 1);
    assert!(root.windows().unwrap().get(0).is_none());
}
#[test]
fn invalid_insert_creates_list_before_bounds_failure_and_event_failure_keeps_null() {
    let f = WindowsFactory::new();
    let root = RootDock::new();
    assert!(catch_unwind(AssertUnwindSafe(|| f.insert_window(
        Some(root.clone()),
        None,
        1
    )))
    .is_err());
    assert_eq!(root.windows().unwrap().count(), 0);
    assert!(f.trace.borrow().is_empty());
    f.action.replace(Some(Rc::new(|| panic!("added aborted"))));
    assert_eq!(
        failure(|| f.add_window(Some(root.clone()), None)),
        "added aborted"
    );
    assert_eq!(&*f.trace.borrow(), &["added:null"]);
    assert_eq!(root.windows().unwrap().count(), 1);
    assert!(f.windows.borrow().is_empty());
}
#[test]
fn added_callback_replaces_live_list_while_initialization_keeps_original_null_and_owner() {
    let f = WindowsFactory::new();
    let root = RootDock::new();
    let old: ListHandle<Option<Rc<dyn IDockWindow>>> = ListHandle::new();
    let replacement = ListHandle::new();
    root.set_windows(Some(old.clone()));
    let weak = Rc::downgrade(&root);
    let next = replacement.clone();
    f.action.replace(Some(Rc::new(move || {
        weak.upgrade().unwrap().set_windows(Some(next.clone()))
    })));
    f.add_window(Some(root.clone()), None);
    assert_eq!(old.count(), 1);
    assert_eq!(replacement.count(), 0);
    assert!(root.windows().unwrap().ptr_eq(&replacement));
    assert_eq!(&*f.trace.borrow(), &["added:null", "init:null"]);
    assert!(Rc::ptr_eq(
        f.owners.borrow()[0].as_ref().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
}
#[test]
fn host_factory_receives_null_before_explicit_override_or_default_host_assignment_failure() {
    let f = WindowsFactory::new();
    f.init_base.set(true);
    f.host_enabled.set(true);
    let host = RecordingHost::new();
    f.host.replace(Some(host.clone()));
    let owner = RootDock::new();
    f.init_dock_window(None, Some(owner.clone()));
    assert_eq!(
        &*f.trace.borrow(),
        &["init:null", "host:null", "explicit:null"]
    );
    assert!(host.calls.borrow().is_empty());
    assert!(Rc::ptr_eq(
        f.owners.borrow().last().unwrap().as_ref().unwrap(),
        &(owner.clone() as Rc<dyn IDockable>)
    ));
    f.trace.borrow_mut().clear();
    f.explicit_base.set(true);
    assert!(failure(|| f.init_dock_window(None, Some(owner))).contains("window.Host"));
    assert_eq!(
        &*f.trace.borrow(),
        &["init:null", "host:null", "explicit:null"]
    );
    assert!(host.calls.borrow().is_empty());
}
#[test]
fn root_initialization_forwards_null_window_entries_in_original_order() {
    let f = WindowsFactory::new();
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_id("second".into());
    let list = ListHandle::from_items(vec![None, Some(window.clone() as Rc<dyn IDockWindow>)]);
    root.set_windows(Some(list.clone()));
    f.component.base().init_dockable(Some(root.clone()), None);
    assert_eq!(&*f.trace.borrow(), &["init:null", "init:second"]);
    assert!(root.windows().unwrap().ptr_eq(&list));
    assert!(f.windows.borrow()[0].is_none());
    assert!(Rc::ptr_eq(
        f.windows.borrow()[1].as_ref().unwrap(),
        &(window as Rc<dyn IDockWindow>)
    ));
}
#[test]
fn show_and_exit_fail_at_null_after_first_window_side_effects_without_visiting_later_window() {
    for exit in [false, true] {
        let root = RootDock::new();
        let first = TestWindow::new();
        let last = TestWindow::new();
        root.set_windows(Some(ListHandle::from_items(vec![
            Some(first.clone() as Rc<dyn IDockWindow>),
            None,
            Some(last.clone() as Rc<dyn IDockWindow>),
        ])));
        let error = failure(|| {
            let adapter = ferrodock_model::adapters::NavigateAdapter::new(root.clone());
            if exit {
                adapter.exit_windows()
            } else {
                adapter.show_windows()
            };
        });
        assert!(error.contains(if exit {
            "window.Save"
        } else {
            "window.IsModal"
        }));
        assert_eq!(
            &*first.window.calls.borrow(),
            if exit {
                &["Save", "Exit"][..]
            } else {
                &["Present"][..]
            }
        );
        assert!(last.window.calls.borrow().is_empty());
    }
}
#[test]
fn search_and_state_traversal_reject_null_window_before_later_layout() {
    let f = Factory::new();
    let root = RootDock::new();
    let later = DockWindow::new();
    let layout = RootDock::new();
    later.set_layout(Some(layout));
    root.set_windows(Some(ListHandle::from_items(vec![
        None,
        Some(later as Rc<dyn IDockWindow>),
    ])));
    assert!(failure(|| {
        f.find_dockable(Some(root.clone()), Some(Rc::new(|_| false)));
    })
    .contains("window.Layout"));
    assert!(failure(|| DockState::new().save(root.clone())).contains("window.Layout"));
}
#[test]
fn explicit_initialization_preserves_layout_argument_evaluation_before_virtual_init() {
    for second_owner_null in [false, true] {
        let f = WindowsFactory::new();
        f.explicit_base.set(true);
        let window = TestWindow::new();
        window.set_id("window".into());
        window.window.calls.borrow_mut().clear();
        let a = RootDock::new();
        let b = RootDock::new();
        let owner = ToolDock::new();
        b.set_owner(Some(owner.clone()));
        let reads = Rc::new(Cell::new(0));
        let count = reads.clone();
        let sequence = Rc::new(RefCell::new(VecDeque::from([
            Some(a as Rc<dyn IRootDock>),
            None,
            if second_owner_null {
                None
            } else {
                Some(b as Rc<dyn IRootDock>)
            },
        ])));
        window.window.layout_getter.replace(Some(Rc::new(move || {
            count.set(count.get() + 1);
            sequence
                .borrow_mut()
                .pop_front()
                .expect("source layout read")
        })));
        if second_owner_null {
            assert!(
                failure(|| f.init_dock_window_with_host(Some(window.clone()), None, None))
                    .contains("source layout access")
            );
            assert!(f.init_items.borrow().is_empty());
        } else {
            f.init_dock_window_with_host(Some(window.clone()), None, None);
            assert_eq!(f.init_items.borrow().len(), 1);
            assert!(f.init_items.borrow()[0].is_none());
            assert!(Rc::ptr_eq(
                f.owners.borrow().last().unwrap().as_ref().unwrap(),
                &(owner as Rc<dyn IDockable>)
            ));
        }
        assert_eq!(reads.get(), 3);
        assert!(window.factory().is_some());
        assert_eq!(
            &window.window.calls.borrow()[..3],
            &["Host", "Owner", "Factory"]
        );
    }
}

struct NullCollectionRootFactory {
    component: Factory,
    root: Rc<dyn IRootDock>,
    window: Rc<dyn IDockWindow>,
    resolved: Cell<usize>,
    added: Cell<usize>,
    removed: Cell<usize>,
}
impl NullCollectionRootFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: RootDock::new(),
            window: DockWindow::new(),
            resolved: Cell::new(0),
            added: Cell::new(0),
            removed: Cell::new(0),
        })
    }
}
impl FactoryBaseImpl for NullCollectionRootFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn resolve_window_collection_root(
        &self,
        root: Option<Rc<dyn IRootDock>>,
    ) -> Option<Rc<dyn IRootDock>> {
        assert!(Rc::ptr_eq(root.as_ref().unwrap(), &self.root));
        self.resolved.set(self.resolved.get() + 1);
        None
    }
}
impl FactoryImpl for NullCollectionRootFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        _: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        Some(this.root.clone())
    }
    fn remove_dockable(this: &Self, item: Option<Rc<dyn IDockable>>, collapse: bool) {
        assert!(item.is_none() && collapse);
        this.removed.set(this.removed.get() + 1);
    }
    fn create_window_from_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        assert!(item.is_none() && options.is_none());
        Some(this.window.clone())
    }
    fn add_window(
        this: &Self,
        root: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
    ) {
        assert!(root.is_none());
        assert!(Rc::ptr_eq(window.as_ref().unwrap(), &this.window));
        this.added.set(this.added.get() + 1);
    }
}
impl_factory!(NullCollectionRootFactory);
#[test]
fn null_collection_root_override_flows_to_actual_add_override_without_early_rejection() {
    let f = NullCollectionRootFactory::new();
    f.split_to_window(None, None, 1., 2., 300., 400.);
    assert_eq!(f.resolved.get(), 1);
    assert_eq!(f.added.get(), 1);
    assert_eq!(f.removed.get(), 1);
    assert_eq!(
        (
            f.window.x(),
            f.window.y(),
            f.window.width(),
            f.window.height()
        ),
        (1., 2., 300., 400.)
    );
    assert!(f.root.windows().is_none());
}
