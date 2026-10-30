//! Original nullable focus/activation slots and callback-driven selection/window changes.
use crate::test_host::{HostCall, RecordingHost};
use crate::{
    controls::*,
    core::{DockWindow, DockableBase, DockableBaseImpl},
    impl_dockable, impl_factory, Factory, FactoryImpl,
};
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferroui_reactiveui::reactive_object;
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
type Trace = Rc<RefCell<Vec<String>>>;
type Action = Rc<dyn Fn()>;
fn name(item: Option<Rc<dyn IDockable>>) -> String {
    item.map_or_else(|| "null".into(), |item| item.id())
}
fn failure(action: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(action)).expect_err("source dereference failure");
    error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
struct FocusFactory {
    component: Factory,
    trace: Trace,
    root: RefCell<Option<Rc<dyn IRootDock>>>,
    allow_focus: Cell<bool>,
    last_dock: RefCell<Option<Rc<dyn IDock>>>,
    last_item: RefCell<Option<Rc<dyn IDockable>>>,
}
impl FocusFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            trace: Default::default(),
            root: Default::default(),
            allow_focus: Cell::new(true),
            last_dock: Default::default(),
            last_item: Default::default(),
        })
    }
}
impl FactoryBaseImpl for FocusFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn should_focus_active_dockable(
        &self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDock>>,
    ) -> bool {
        self.trace.borrow_mut().push(format!(
            "gate:{}:{}",
            name(item),
            name(owner.map(|owner| owner as Rc<dyn IDockable>))
        ));
        self.allow_focus.get()
    }
}
impl FactoryImpl for FocusFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        this.trace.borrow_mut().push(format!(
            "find:{}:{}",
            name(item),
            if predicate.is_some() { "focus" } else { "all" }
        ));
        this.root.borrow().clone()
    }
    fn on_active_dockable_changed(this: &Self, item: Option<Rc<dyn IDockable>>) {
        this.trace
            .borrow_mut()
            .push(format!("active:{}", name(item.clone())));
        this.component.base().on_active_dockable_changed(item);
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.trace.borrow_mut().push(format!(
            "init:{}:{}",
            name(item.clone()),
            name(owner.clone())
        ));
        this.component.base().init_dockable(item, owner);
    }
    fn set_focused_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
    ) {
        this.trace.borrow_mut().push(format!(
            "focus:{}:{}",
            name(dock.clone().map(|dock| dock as Rc<dyn IDockable>)),
            name(item.clone())
        ));
        this.last_dock.replace(dock);
        this.last_item.replace(item);
    }
    fn on_window_activated(this: &Self, window: Option<Rc<dyn IDockWindow>>) {
        this.trace.borrow_mut().push(format!(
            "window:{}",
            window
                .as_ref()
                .map_or_else(|| "null".into(), |window| window.id())
        ));
        this.component.base().on_window_activated(window);
    }
}
impl_factory!(FocusFactory);
struct SelectedItem {
    base: DockableBase,
    trace: Trace,
    action: Option<Action>,
}
reactive_object!(SelectedItem, base);
impl DockableBaseImpl for SelectedItem {
    fn on_selected(this: &Self) {
        this.trace.borrow_mut().push("selected".into());
        if let Some(action) = &this.action {
            action();
        }
    }
}
impl_dockable!(SelectedItem);
impl SelectedItem {
    fn new(trace: Trace, action: Option<Action>) -> Rc<Self> {
        let item = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| Self {
            base: DockableBase::construct(weak.clone()),
            trace,
            action,
        });
        item.set_id("item".into());
        item
    }
}

#[test]
fn default_null_inputs_fail_at_original_lookup_or_final_selection_after_active_event() {
    let factory = Factory::new();
    assert!(FactoryBaseImpl::should_focus_active_dockable(
        factory.as_ref(),
        None,
        None
    ));
    assert!(failure(|| factory.set_focused_dockable(None, None)).contains("source dockable.Owner"));
    assert!(failure(|| factory.activate_window(None)).contains("source dockable.Owner"));
    let events = Rc::new(Cell::new(0));
    let count = events.clone();
    factory.active_dockable_changed(Rc::new(move |e| {
        assert!(e.dockable().is_none());
        count.set(count.get() + 1);
    }));
    assert!(failure(|| factory.init_active_dockable(None, None)).contains("owner.VisibleDockables"));
    assert_eq!(events.get(), 1);
    let doc = Document::new();
    let owner = ToolDock::new();
    doc.set_owner(Some(owner.clone()));
    assert!(Rc::ptr_eq(
        &doc.owner().unwrap(),
        &(owner.clone() as Rc<dyn IDockable>)
    ));
    let ordinary = Factory::new();
    assert!(
        failure(|| ordinary.init_active_dockable(Some(doc.clone()), None))
            .contains("source dockable.Owner")
    );
    assert!(doc.owner().is_none());
    assert!(Rc::ptr_eq(
        &doc.factory().unwrap(),
        &(ordinary as Rc<dyn IFactory>)
    ));
}
#[test]
fn nullable_focus_and_activation_lookup_can_return_no_root_without_rejecting_null() {
    let f = FocusFactory::new();
    f.component.base().set_focused_dockable(None, None);
    f.activate_window(None);
    assert_eq!(&*f.trace.borrow(), &["find:null:focus", "find:null:all"]);
    let dock = ToolDock::new();
    f.init_active_dockable(None, Some(dock));
    assert_eq!(f.trace.borrow().last().unwrap(), "active:null");
}
#[test]
fn active_selection_keeps_initialization_selected_gate_and_focus_before_null_owner_failure() {
    for suppress in [false, true] {
        let f = FocusFactory::new();
        let weak = Rc::downgrade(&f);
        let item = SelectedItem::new(
            f.trace.clone(),
            Some(Rc::new(move || {
                weak.upgrade().unwrap().allow_focus.set(!suppress)
            })),
        );
        assert!(failure(|| f.init_active_dockable(Some(item.clone()), None))
            .contains("owner.VisibleDockables"));
        let mut expected = vec![
            "active:item",
            "find:item:focus",
            "find:item:focus",
            "init:item:null",
            "find:item:focus",
            "find:item:focus",
            "selected",
            "gate:item:null",
        ];
        if !suppress {
            expected.push("focus:null:item");
            assert!(Rc::ptr_eq(
                f.last_item.borrow().as_ref().unwrap(),
                &(item.clone() as Rc<dyn IDockable>)
            ));
        }
        assert_eq!(&*f.trace.borrow(), &expected);
        assert!(f.last_dock.borrow().is_none());
        assert!(item.owner().is_none());
        assert!(Rc::ptr_eq(
            &item.factory().unwrap(),
            &(f as Rc<dyn IFactory>)
        ));
    }
}
#[test]
fn selected_override_failure_aborts_before_focus_policy_and_final_owner_access() {
    let f = FocusFactory::new();
    let item = SelectedItem::new(
        f.trace.clone(),
        Some(Rc::new(|| panic!("selected aborted"))),
    );
    assert_eq!(
        failure(|| f.init_active_dockable(Some(item.clone()), None)),
        "selected aborted"
    );
    assert_eq!(
        &*f.trace.borrow(),
        &[
            "active:item",
            "find:item:focus",
            "find:item:focus",
            "init:item:null",
            "find:item:focus",
            "find:item:focus",
            "selected"
        ]
    );
    assert!(item.factory().is_some() && item.owner().is_none());
}
#[test]
fn actual_host_activation_rereads_cleared_or_replaced_root_window_and_raises_live_payload() {
    for replace in [false, true] {
        let f = FocusFactory::new();
        let root = RootDock::new();
        let window = DockWindow::new();
        let next = DockWindow::new();
        next.set_id("next".into());
        root.set_window(Some(window.clone()));
        f.root.replace(Some(root.clone()));
        let host = RecordingHost::new();
        window.set_host(Some(host.clone()));
        let weak = Rc::downgrade(&root);
        let replacement = next.clone();
        host.on_call.replace(Some(Rc::new(move |call| {
            if *call == HostCall::Active {
                weak.upgrade().unwrap().set_window(if replace {
                    Some(replacement.clone())
                } else {
                    None
                });
            }
        })));
        let received = Rc::new(RefCell::new(Vec::new()));
        let events = received.clone();
        f.window_activated(Rc::new(move |args| events.borrow_mut().push(args.window())));
        f.activate_window(None);
        assert_eq!(&*host.calls.borrow(), &[HostCall::Active]);
        assert_eq!(
            &*f.trace.borrow(),
            &[
                "find:null:all",
                if replace {
                    "window:next"
                } else {
                    "window:null"
                }
            ]
        );
        let received = received.borrow();
        assert_eq!(received.len(), 1);
        if replace {
            assert!(Rc::ptr_eq(
                received[0].as_ref().unwrap(),
                &(next as Rc<dyn IDockWindow>)
            ));
        } else {
            assert!(received[0].is_none());
        }
        assert!(window.host().is_some());
    }
}
#[test]
fn nullable_focus_root_override_commits_focus_before_final_null_selection_owner_failure() {
    let f = FocusFactory::new();
    let root = RootDock::new();
    f.root.replace(Some(root.clone()));
    let item = Document::new();
    item.set_id("doc".into());
    assert!(failure(|| f
        .component
        .base()
        .set_focused_dockable(None, Some(item.clone())))
    .contains("owner.VisibleDockables"));
    assert!(Rc::ptr_eq(
        &root.focused_dockable().unwrap(),
        &(item as Rc<dyn IDockable>)
    ));
    assert_eq!(f.trace.borrow().first().unwrap(), "find:null:focus");
}
