use super::*;
use crate::overlays::hosting::hosting_test_support::{Overlays, Screen};
use crate::test_environment::Environment;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_reactive::ObservableExt;
use ferroui_system::threading::tasks::{TaskCompletionSource, TaskWorkError};
use std::cell::RefCell;

struct ReloadRoot {
    base: RoutableRootDock,
    calls: Cell<usize>,
    work: RefCell<Task<()>>,
}
reactive_object!(ReloadRoot, base);
impl DockableBaseImpl for ReloadRoot {}
impl_dock!(@base ReloadRoot,RoutableRootDock,
    fn as_root_dock(self:Rc<Self>)->Option<Rc<dyn IRootDock>> {Some(self)},
    fn boxed_interface(self:Rc<Self>,target:TypeId)->Option<BoxedValue> {
        if target==TypeId::of::<dyn IScreen>() {return Some(Rc::new(self as Rc<dyn IScreen>));}
        if target==TypeId::of::<dyn IRoutableViewModel>() {return Some(Rc::new(self as Rc<dyn IRoutableViewModel>));}
        if target==TypeId::of::<dyn IReloadable>() {return Some(Rc::new(self as Rc<dyn IReloadable>));} None
    }
);
root_contracts!(ReloadRoot);
impl ReloadRoot {
    fn new(host: Rc<dyn IScreen>) -> Rc<Self> {
        let root = Rc::new_cyclic(|weak| Self {
            base: RoutableRootDock::construct(weak, host, None),
            calls: Cell::new(0),
            work: RefCell::new(Task::from_result(())),
        });
        root.base.initialize(&root);
        root
    }
    fn register(self: &Rc<Self>, types: &HostTypeRegistry) {
        types.register_dockable_screen(self);
        types.register_routable(self);
    }
}
impl IScreen for ReloadRoot {
    fn router(&self) -> Rc<RoutingState> {
        self.base.router()
    }
}
impl IRoutableViewModel for ReloadRoot {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.base.url_path_segment())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.base.host_screen()
    }
}
impl IReloadable for ReloadRoot {
    fn reload_async(&self) -> Task<()> {
        self.calls.set(self.calls.get() + 1);
        self.work.borrow().clone()
    }
}
fn root(host: Rc<dyn IScreen>, overlays: &Overlays) -> Rc<BusyRootDock> {
    let services = overlays.host.clone();
    BusyRootDock::new(host, Rc::new(move || services.clone()), None)
}
#[test]
fn reload_empty_and_plain_content_completes_without_extra_busy_scope() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let root = root(host, &overlays);
    assert!(root.busy().can_reload());
    assert!(!root.busy().is_busy());
    assert!(root.reload_current_async().is_completed_successfully());
    root.set_active_dockable(Some(Document::new()));
    assert!(root.reload_current_async().is_completed_successfully());
    assert!(!root.busy().is_busy());
}
#[test]
fn active_child_short_circuits_parent_reload_and_routes_even_when_child_is_not_reloadable() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let root = root(host.clone(), &overlays);
    let parent = ReloadRoot::new(host.clone());
    let routed = ReloadRoot::new(host.clone());
    let child = ReloadRoot::new(host);
    parent.register(&root.types);
    routed.register(&root.types);
    child.register(&root.types);
    let _execution = parent
        .router()
        .navigate()
        .execute(Some(routed.clone()))
        .subscribe_next(|_| {});
    parent.set_active_dockable(Some(Document::new()));
    root.set_active_dockable(Some(parent.clone()));
    assert!(root.reload_current_async().is_completed_successfully());
    assert_eq!(parent.calls.get(), 0);
    assert_eq!(routed.calls.get(), 0);
    parent.set_active_dockable(Some(child.clone()));
    assert!(root.reload_current_async().is_completed_successfully());
    assert_eq!(child.calls.get(), 1);
    assert_eq!(parent.calls.get(), 0);
    assert_eq!(routed.calls.get(), 0);
    assert_eq!(parent.base.url_path_segment(), "ReloadRoot");
}
#[test]
fn only_top_router_entry_is_examined_then_screen_self_is_used() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let root = root(host.clone(), &overlays);
    let parent = ReloadRoot::new(host.clone());
    let routed = ReloadRoot::new(host.clone());
    parent.register(&root.types);
    routed.register(&root.types);
    let _first = parent
        .router()
        .navigate()
        .execute(Some(routed.clone()))
        .subscribe_next(|_| {});
    root.set_active_dockable(Some(parent.clone()));
    assert!(root.reload_current_async().is_completed_successfully());
    assert_eq!(routed.calls.get(), 1);
    assert_eq!(parent.calls.get(), 0);
    let plain = crate::navigation::controls::RoutableDocument::new(host, None);
    let _second = parent
        .router()
        .navigate()
        .execute(Some(plain.clone()))
        .subscribe_next(|_| {});
    assert!(root.reload_current_async().is_completed_successfully());
    assert_eq!(routed.calls.get(), 1);
    assert_eq!(parent.calls.get(), 1);
    assert!(crate::navigation::services::DockNavigationHelpers::try_navigate_back(parent.clone()));
    assert!(root.reload_current_async().is_completed_successfully());
    assert_eq!(routed.calls.get(), 2);
    assert_eq!(parent.calls.get(), 1);
}
#[test]
fn reload_retains_pending_task_and_propagates_original_failure() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let root = root(host.clone(), &overlays);
    let reload = ReloadRoot::new(host);
    reload.register(&root.types);
    root.set_active_dockable(Some(reload.clone()));
    let pending = TaskCompletionSource::new();
    reload.work.replace(pending.task());
    let task = root.reload_current_async();
    assert!(!task.is_completed());
    assert_eq!(reload.calls.get(), 1);
    pending.set_result(());
    assert!(task.is_completed_successfully());
    let failure: Rc<dyn std::error::Error> = Rc::new(TaskWorkError {
        message: "reload failed".into(),
    });
    reload.work.replace(Task::from_exception(failure.clone()));
    let failed = root.reload_current_async();
    assert!(Rc::ptr_eq(&failed.result().unwrap_err(), &failure));
}
#[test]
fn actual_busy_command_reloads_and_dispose_clears_it_before_cancelling_pending_overlays_once() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let root = root(host.clone(), &overlays);
    let reload = ReloadRoot::new(host);
    reload.register(&root.types);
    root.set_active_dockable(Some(reload.clone()));
    root.busy().reload_command().execute(None);
    assert_eq!(reload.calls.get(), 1);
    let dialog = overlays
        .dialogs
        .inner
        .show_async_boxed(DialogContent::new(Rc::new(1)), None);
    let confirmation = overlays.confirmations.inner.confirm_async(
        "title".into(),
        "message".into(),
        "ok".into(),
        "cancel".into(),
    );
    assert!(!dialog.is_completed());
    assert!(!confirmation.is_completed());
    let log = overlays.log.clone();
    root.busy().property_changed().add(Rc::new(move |name| {
        if name == "CanReload" {
            log.borrow_mut().push("reload");
        }
    }));
    root.dispose();
    root.dispose();
    assert_eq!(
        &*overlays.log.borrow(),
        &["reload", "dialogs", "confirmations"]
    );
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(overlays.confirmations.calls.get(), 1);
    assert!(dialog.result().unwrap().is_none());
    assert!(!confirmation.result().unwrap());
    assert!(!root.busy().can_reload());
    assert!(!root.busy().reload_command().can_execute(None));
}
#[test]
fn dispose_marks_before_reentry_and_failure_and_does_not_retry_skipped_cancellation() {
    let _env = Environment::new();
    let host = Screen::new("host");
    let overlays = Overlays::new();
    let root = root(host, &overlays);
    let weak = Rc::downgrade(&root);
    root.busy().property_changed().add(Rc::new(move |name| {
        if name == "CanReload" {
            weak.upgrade().unwrap().dispose();
        }
    }));
    overlays.dialogs.fail.set(true);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| root.dispose())).is_err());
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(overlays.confirmations.calls.get(), 0);
    overlays.dialogs.fail.set(false);
    root.dispose();
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(overlays.confirmations.calls.get(), 0);
    assert!(!root.busy().can_reload());
}
