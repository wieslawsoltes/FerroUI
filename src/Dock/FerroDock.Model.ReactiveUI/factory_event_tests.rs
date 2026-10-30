use crate::core::{DockWindow, DockWindowImpl, DockableBase, DockableBaseImpl};
use crate::factory_init_tests::factory;
use crate::factory_tracking_tests::Scene;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::services::IWindowLifecycleService;
use ferroui_reactiveui::reactive_object;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

struct ProbeWindow {
    base: DockWindow,
    allow_close: Cell<bool>,
    close_calls: Cell<usize>,
    calls: RefCell<Vec<&'static str>>,
}
reactive_object!(ProbeWindow, base);
crate::impl_dock_window!(ProbeWindow);
impl DockWindowImpl for ProbeWindow {
    fn on_close(this: &Self) -> bool {
        this.close_calls.set(this.close_calls.get() + 1);
        this.allow_close.get()
    }
    fn on_move_drag_begin(this: &Self) -> bool {
        this.calls.borrow_mut().push("begin");
        false
    }
    fn on_move_drag(this: &Self) {
        this.calls.borrow_mut().push("move");
    }
    fn on_move_drag_end(this: &Self) {
        this.calls.borrow_mut().push("end");
    }
}
impl ProbeWindow {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DockWindow::construct(weak.clone()),
            allow_close: Cell::new(true),
            close_calls: Cell::new(0),
            calls: RefCell::new(Vec::new()),
        })
    }
}

struct ProbeDockable {
    base: DockableBase,
    close_calls: Cell<usize>,
}
reactive_object!(ProbeDockable, base);
crate::impl_dockable!(ProbeDockable);
impl DockableBaseImpl for ProbeDockable {
    fn on_close(this: &Self) -> bool {
        this.close_calls.set(this.close_calls.get() + 1);
        false
    }
}
impl ProbeDockable {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DockableBase::construct(weak.clone()),
            close_calls: Cell::new(0),
        })
    }
}

type ServiceCallback = Rc<dyn Fn(bool, Option<Rc<dyn IDockWindow>>)>;
struct Service {
    callback: ServiceCallback,
}
impl IWindowLifecycleService for Service {
    fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        (self.callback)(true, window);
    }
    fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        (self.callback)(false, window);
    }
}

#[test]
fn mutation_events_raise_before_reconciling_the_observers_current_layout() {
    macro_rules! check {
        ($subscribe:ident, $notify:ident) => {{
            let factory = factory();
            let scene = Scene::new();
            let base: &dyn IFactoryEvents = factory.base.get().unwrap();
            let observed = Rc::new(Cell::new(false));
            let called = observed.clone();
            let root = scene.root.clone();
            let item = scene.document.clone();
            base.$subscribe(Rc::new(move |args| {
                assert!(Rc::ptr_eq(
                    &args.dockable().unwrap(),
                    &(item.clone() as Rc<dyn IDockable>)
                ));
                assert_eq!(item.docking_state(), DockingWindowState::DOCKED);
                root.set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(item.clone())])));
                called.set(true);
            }));
            base.$notify(Some(scene.document.clone()));
            assert!(observed.get());
            assert_eq!(
                scene.document.docking_state(),
                DockingWindowState::DOCUMENT | DockingWindowState::HIDDEN
            );
        }};
    }
    check!(dockable_added, on_dockable_added);
    check!(dockable_removed, on_dockable_removed);
    check!(dockable_closed, on_dockable_closed);
    check!(dockable_pinned, on_dockable_pinned);
    check!(dockable_unpinned, on_dockable_unpinned);
    check!(dockable_hidden, on_dockable_hidden);
    check!(dockable_restored, on_dockable_restored);
}

#[test]
fn move_swap_and_operation_events_keep_source_payloads_without_implicit_state_updates() {
    let factory = factory();
    let scene = Scene::new();
    let base: &dyn IFactoryEvents = factory.base.get().unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    base.dockable_moved(Rc::new(move |args| {
        assert!(args.dockable().is_some());
        log.borrow_mut().push("moved");
    }));
    let log = calls.clone();
    base.dockable_swapped(Rc::new(move |args| {
        assert!(args.dockable().is_none());
        log.borrow_mut().push("swapped");
    }));
    let log = calls.clone();
    base.dockable_docked(Rc::new(move |args| {
        assert_eq!(args.operation(), DockOperation::OuterLeft);
        log.borrow_mut().push("docked");
    }));
    let log = calls.clone();
    base.dockable_undocked(Rc::new(move |args| {
        assert_eq!(args.operation(), DockOperation::Window);
        log.borrow_mut().push("undocked");
    }));
    base.on_dockable_moved(Some(scene.document.clone()));
    base.on_dockable_swapped(None);
    base.on_dockable_docked(Some(scene.document.clone()), DockOperation::OuterLeft);
    base.on_dockable_undocked(None, DockOperation::Window);
    assert_eq!(*calls.borrow(), ["moved", "swapped", "docked", "undocked"]);
    assert_eq!(scene.document.docking_state(), DockingWindowState::DOCKED);
}

#[test]
fn close_policy_preserves_content_hook_but_still_allows_factory_cancellation() {
    for preserve in [false, true] {
        for cancel in [false, true] {
            let factory = factory();
            factory.supports_close_policy.set(true);
            factory.preserve_layout.set(preserve);
            let window = ProbeWindow::new();
            let expected = window.clone();
            let notifications = Rc::new(Cell::new(0));
            let count = notifications.clone();
            factory.window_closing(Rc::new(move |args| {
                assert!(Rc::ptr_eq(
                    &args.window().unwrap(),
                    &(expected.clone() as Rc<dyn IDockWindow>)
                ));
                count.set(count.get() + 1);
                args.set_cancel(cancel);
            }));
            assert_eq!(factory.on_window_closing(Some(window.clone())), !cancel);
            assert_eq!(window.close_calls.get(), usize::from(!preserve));
            assert_eq!(notifications.get(), 1);
            assert!(Rc::ptr_eq(
                &factory.close_policy_requests.borrow()[0],
                &(window as Rc<dyn IDockWindow>)
            ));
        }
    }
}

#[test]
fn ordinary_close_veto_and_null_defaults_remain_overridable_by_shared_cancellation() {
    let factory = factory();
    let window = ProbeWindow::new();
    window.allow_close.set(false);
    assert!(!factory.on_window_closing(Some(window.clone())));
    assert_eq!(window.close_calls.get(), 1);
    assert!(factory.on_window_closing(None));
    assert!(factory.close_policy_requests.borrow().is_empty());
    factory.window_closing(Rc::new(|args| {
        assert!(args.cancel());
        args.clone().set_cancel(false);
    }));
    assert!(factory.on_window_closing(Some(window)));
    let item = ProbeDockable::new();
    assert!(!factory.on_dockable_closing(Some(item.clone())));
    assert_eq!(item.close_calls.get(), 1);
    assert!(factory.on_dockable_closing(None));
    factory.dockable_closing(Rc::new(|args| args.clone().set_cancel(false)));
    assert!(factory.on_dockable_closing(Some(item.clone())));
    assert_eq!(item.close_calls.get(), 2);
}

#[test]
fn drag_callbacks_run_before_events_and_final_cancellation_can_override_model_veto() {
    let factory = factory();
    let window = ProbeWindow::new();
    let model = window.clone();
    factory.window_move_drag_begin(Rc::new(move |args| {
        assert_eq!(*model.calls.borrow(), ["begin"]);
        assert!(args.cancel());
        args.set_cancel(false);
    }));
    let model = window.clone();
    factory.window_move_drag(Rc::new(move |_| {
        assert_eq!(*model.calls.borrow(), ["begin", "move"])
    }));
    let model = window.clone();
    factory.window_move_drag_end(Rc::new(move |_| {
        assert_eq!(*model.calls.borrow(), ["begin", "move", "end"])
    }));
    assert!(factory.on_window_move_drag_begin(Some(window.clone())));
    factory.on_window_move_drag(Some(window.clone()));
    factory.on_window_move_drag_end(Some(window));
}

#[test]
fn null_window_open_added_and_drag_notifications_preserve_null_payloads() {
    let factory = factory();
    let count = Rc::new(Cell::new(0));
    macro_rules! subscribe {
        ($event:ident) => {{
            let count = count.clone();
            factory.$event(Rc::new(move |args| {
                assert!(args.window().is_none());
                count.set(count.get() + 1);
            }));
        }};
    }
    subscribe!(window_added);
    subscribe!(window_opened);
    subscribe!(window_move_drag);
    subscribe!(window_move_drag_end);
    factory.on_window_added(None);
    factory.on_window_opened(None);
    factory.on_window_move_drag(None);
    factory.on_window_move_drag_end(None);
    assert!(factory.on_window_move_drag_begin(None));
    assert_eq!(count.get(), 4);
}

#[test]
fn closed_and_removed_order_is_tracking_then_event_then_live_services() {
    for closed in [true, false] {
        let factory = factory();
        let scene = Scene::new();
        factory.on_window_activated(Some(scene.window.clone()));
        let log = Rc::new(RefCell::new(Vec::new()));
        let events = log.clone();
        factory.global_dock_tracking_changed(Rc::new(move |args| {
            assert!(args.current().root_dock().is_none());
            events.borrow_mut().push("tracking");
        }));
        let events = log.clone();
        if closed {
            factory.window_closed(Rc::new(move |_| events.borrow_mut().push("event")));
        } else {
            factory.window_removed(Rc::new(move |_| events.borrow_mut().push("event")));
        }
        let services = factory.base.get().unwrap().window_lifecycle_services();
        assert!(services == factory.base.get().unwrap().window_lifecycle_services());
        for name in ["first", "second"] {
            let events = log.clone();
            let expected = scene.window.clone();
            services.add(Some(Rc::new(Service {
                callback: Rc::new(move |was_closed, window| {
                    assert_eq!(was_closed, closed);
                    assert!(Rc::ptr_eq(
                        &window.unwrap(),
                        &(expected.clone() as Rc<dyn IDockWindow>)
                    ));
                    events.borrow_mut().push(name);
                }),
            })));
            services.add(None);
        }
        if closed {
            factory.on_window_closed(Some(scene.window));
        } else {
            factory.on_window_removed(Some(scene.window));
        }
        assert_eq!(*log.borrow(), ["tracking", "event", "first", "second"]);
    }
}

#[test]
fn lifecycle_callbacks_forward_null_and_repeat_notifications_without_factory_deduplication() {
    let factory = factory();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    factory
        .base
        .get()
        .unwrap()
        .window_lifecycle_services()
        .add(Some(Rc::new(Service {
            callback: Rc::new(move |closed, window| {
                assert!(window.is_none());
                log.borrow_mut().push(closed);
            }),
        })));
    factory.on_window_closed(None);
    factory.on_window_removed(None);
    factory.on_window_closed(None);
    assert_eq!(*calls.borrow(), [true, false, true]);
}

#[test]
fn protected_lifecycle_overrides_dispatch_instead_of_default_service_loop() {
    let factory = factory();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    *factory.on_notify_closed.borrow_mut() =
        Some(Rc::new(move |_| log.borrow_mut().push("closed")));
    let log = calls.clone();
    *factory.on_notify_removed.borrow_mut() =
        Some(Rc::new(move |_| log.borrow_mut().push("removed")));
    factory
        .base
        .get()
        .unwrap()
        .window_lifecycle_services()
        .add(Some(Rc::new(Service {
            callback: Rc::new(|_, _| panic!("overridden service loop must not execute")),
        })));
    factory.on_window_closed(None);
    factory.on_window_removed(None);
    assert_eq!(*calls.borrow(), ["closed", "removed"]);
}

#[test]
fn lifecycle_list_mutation_fails_fast_after_callback_and_can_be_retried() {
    let factory = factory();
    let services = factory.base.get().unwrap().window_lifecycle_services();
    let owner = Rc::downgrade(&factory);
    let mutated = Cell::new(false);
    services.add(Some(Rc::new(Service {
        callback: Rc::new(move |_, _| {
            if !mutated.replace(true) {
                owner
                    .upgrade()
                    .unwrap()
                    .base
                    .get()
                    .unwrap()
                    .window_lifecycle_services()
                    .add(None);
            }
        }),
    })));
    assert!(catch_unwind(AssertUnwindSafe(|| factory.on_window_closed(None))).is_err());
    factory.on_window_closed(None); // No internal borrow or guard survives the failed enumeration.
}

#[test]
fn closed_event_failure_keeps_tracking_clear_and_stops_service_notification() {
    let factory = factory();
    let scene = Scene::new();
    factory.on_window_activated(Some(scene.window.clone()));
    factory.window_closed(Rc::new(|_| panic!("observer failure")));
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    factory
        .base
        .get()
        .unwrap()
        .window_lifecycle_services()
        .add(Some(Rc::new(Service {
            callback: Rc::new(move |_, _| count.set(count.get() + 1)),
        })));
    assert!(catch_unwind(AssertUnwindSafe(
        || factory.on_window_closed(Some(scene.window))
    ))
    .is_err());
    assert!(factory.current_root_dock().is_none());
    assert_eq!(calls.get(), 0);
}

#[test]
fn lifecycle_service_failure_stops_later_services_after_event_delivery() {
    let factory = factory();
    let order = Rc::new(RefCell::new(Vec::new()));
    let log = order.clone();
    factory.window_removed(Rc::new(move |_| log.borrow_mut().push("event")));
    let log = order.clone();
    let services = factory.base.get().unwrap().window_lifecycle_services();
    services.add(Some(Rc::new(Service {
        callback: Rc::new(move |_, _| {
            log.borrow_mut().push("first");
            panic!("service failure");
        }),
    })));
    services.add(Some(Rc::new(Service {
        callback: Rc::new(|_, _| panic!("second service must not run")),
    })));
    assert!(catch_unwind(AssertUnwindSafe(|| factory.on_window_removed(None))).is_err());
    assert_eq!(*order.borrow(), ["event", "first"]);
}
