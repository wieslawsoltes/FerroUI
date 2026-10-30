use super::*;
use crate::overlays::services::DockGlobalBusyService;
use crate::test_environment::Environment;
use ferroui_reactive::concurrency::HistoricalScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::threading::tasks::{TaskCanceledError, TaskWorkError};

#[test]
fn defaults_scopes_messages_and_reload_visibility_are_independent() {
    let _environment = Environment::new();
    let service: Rc<dyn IDockBusyService> = DockBusyService::new(None, None);
    assert!(!service.is_busy());
    assert!(service.message().is_none());
    assert!(service.is_reload_visible());
    assert!(!service.can_reload());
    assert!(!service.reload_command().can_execute(None));
    service.set_is_reload_visible(false);
    assert!(!service.is_reload_visible());
    let first = service.begin("first".into());
    let second = service.begin("second".into());
    first.dispose();
    first.dispose();
    assert!(service.is_busy());
    assert_eq!(service.message().as_deref(), Some("second"));
    service.update_message(Some("updated".into()));
    assert_eq!(service.message().as_deref(), Some("updated"));
    second.dispose();
    assert!(!service.is_busy());
    assert!(service.message().is_none());
    service.update_message(Some("idle message".into()));
    assert_eq!(service.message().as_deref(), Some("idle message"));
    service.update_message(None);
    assert!(service.message().is_none());
}

#[test]
fn local_notifications_keep_global_begin_and_end_order_and_property_names() {
    let _environment = Environment::new();
    let global = DockGlobalBusyService::new(None, None);
    let service = DockBusyService::new(Some(global.clone()), None);
    let states = Rc::new(RefCell::new(Vec::new()));
    let seen = states.clone();
    let weak = Rc::downgrade(&service);
    let global_probe = global.clone();
    service.property_changed().add(Rc::new(move |name| {
        let service = weak.upgrade().unwrap();
        seen.borrow_mut().push((
            name.to_owned(),
            service.is_busy(),
            service.message(),
            global_probe.is_busy(),
        ));
    }));
    let handle = service.begin("local message".into());
    assert_eq!(global.message().as_deref(), Some("Working..."));
    handle.dispose();
    assert!(!global.is_busy());
    assert_eq!(
        &*states.borrow(),
        &[
            ("IsBusy".into(), true, None, true),
            ("Message".into(), true, Some("local message".into()), true),
            ("IsBusy".into(), false, Some("local message".into()), true),
            ("Message".into(), false, None, true)
        ]
    );
}

#[test]
fn queued_updates_and_handler_installation_use_captured_dispatcher_order() {
    let _environment = Environment::new();
    let scheduler = HistoricalScheduler::historical();
    RxSchedulers::set_main_thread_scheduler(scheduler.clone());
    let global = DockGlobalBusyService::new(None, None);
    let service = DockBusyService::new(Some(global.clone()), None);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    service
        .property_changed()
        .add(Rc::new(move |name| seen.borrow_mut().push(name.to_owned())));
    service.set_reload_handler(Some(Rc::new(|| Task::from_result(()))));
    let handle = service.begin("first".into());
    service.update_message(Some("queued".into()));
    handle.dispose();
    assert!(!service.can_reload());
    assert!(!service.is_busy());
    assert!(changes.borrow().is_empty());
    scheduler.start();
    assert!(service.can_reload());
    assert!(!service.is_busy());
    assert!(!global.is_busy());
    assert!(service.message().is_none());
    assert_eq!(
        &*changes.borrow(),
        &[
            "CanReload",
            "IsBusy",
            "Message",
            "Message",
            "IsBusy",
            "Message"
        ]
    );
}

#[test]
fn run_async_is_eager_holds_pending_scope_and_cleans_before_returned_completion() {
    let _environment = Environment::new();
    let global = DockGlobalBusyService::new(None, None);
    let service = DockBusyService::new(Some(global.clone()), None);
    let work = TaskCompletionSource::new();
    let pending = work.task();
    let called = Rc::new(Cell::new(false));
    let seen = called.clone();
    let task = service.run_async(
        "pending".into(),
        Rc::new(move || {
            seen.set(true);
            pending.clone()
        }),
    );
    assert!(called.get());
    assert!(service.is_busy());
    assert!(global.is_busy());
    assert!(!task.is_completed());
    let observed = Rc::new(Cell::new(false));
    let seen = observed.clone();
    let weak = Rc::downgrade(&service);
    let global_probe = global.clone();
    task.continue_with(move |task| {
        task.result().unwrap();
        assert!(!weak.upgrade().unwrap().is_busy());
        assert!(!global_probe.is_busy());
        seen.set(true);
    });
    work.set_result(());
    assert!(task.is_completed_successfully());
    assert!(observed.get());
}

#[test]
fn run_async_preserves_task_error_identity_and_cancellation_and_handles_synchronous_panics() {
    let _environment = Environment::new();
    let service = DockBusyService::new(None, None);
    let error: ObservableError = Rc::new(TaskWorkError {
        message: "work failed".into(),
    });
    let captured = error.clone();
    let task = service.run_async(
        "error".into(),
        Rc::new(move || Task::from_exception(captured.clone())),
    );
    assert!(task.is_faulted());
    assert!(Rc::ptr_eq(&task.exception().unwrap(), &error));
    assert!(!service.is_busy());
    let task = service.run_async("canceled".into(), Rc::new(Task::from_canceled));
    assert!(task.is_canceled());
    assert!(!service.is_busy());
    assert!(TaskCanceledError::is_cancellation(
        &task.result().unwrap_err()
    ));
    let task = service.run_async("panic".into(), Rc::new(|| panic!("sync action failed")));
    assert!(task.is_faulted());
    assert!(!service.is_busy());
    assert_eq!(task.exception().unwrap().to_string(), "sync action failed");
    let task = service.run_async("success".into(), Rc::new(|| Task::from_result(())));
    assert!(task.is_completed_successfully());
    assert!(!service.is_busy());
}

#[test]
fn reload_command_observes_handler_gate_executes_current_handler_and_tracks_pending_execution() {
    let _environment = Environment::new();
    let service = DockBusyService::new(None, None);
    let command = service.reload_command();
    assert!(Rc::ptr_eq(&command, &service.reload_command()));
    let work = TaskCompletionSource::new();
    let pending = work.task();
    let first = Rc::new(Cell::new(0));
    let calls = first.clone();
    let handler: BusyAction = Rc::new(move || {
        calls.set(calls.get() + 1);
        pending.clone()
    });
    service.set_reload_handler(Some(handler.clone()));
    assert!(service.can_reload());
    assert!(command.can_execute(None));
    service.set_is_reload_visible(false);
    assert!(command.can_execute(None));
    command.execute(None);
    assert_eq!(first.get(), 1);
    assert!(!command.can_execute(None));
    assert!(!service.is_busy());
    let second = Rc::new(Cell::new(0));
    let calls = second.clone();
    service.set_reload_handler(Some(Rc::new(move || {
        calls.set(calls.get() + 1);
        Task::from_result(())
    })));
    work.set_result(());
    assert!(command.can_execute(None));
    command.execute(None);
    assert_eq!(second.get(), 1);
    service.set_reload_handler(None);
    assert!(!service.can_reload());
    assert!(!command.can_execute(None));
}

#[test]
fn reload_replacement_notifications_and_reentrant_handler_changes_release_borrows() {
    let _environment = Environment::new();
    let service = DockBusyService::new(None, None);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let weak = Rc::downgrade(&service);
    let once = Cell::new(false);
    service.property_changed().add(Rc::new(move |name| {
        seen.borrow_mut().push(name.to_owned());
        if name == "CanReload" && !once.replace(true) {
            weak.upgrade().unwrap().set_reload_handler(None);
        }
    }));
    let handler: BusyAction = Rc::new(|| Task::from_result(()));
    service.set_reload_handler(Some(handler.clone()));
    assert!(!service.can_reload());
    assert!(!service.reload_command().can_execute(None));
    assert_eq!(&*changes.borrow(), &["CanReload", "CanReload"]);
    service.set_reload_handler(Some(handler.clone()));
    service.set_reload_handler(Some(handler));
    service.set_reload_handler(Some(Rc::new(|| Task::from_result(()))));
    assert_eq!(changes.borrow().len(), 3);
    service.set_reload_handler(None);
    assert_eq!(changes.borrow().len(), 4);
}

#[test]
fn reload_errors_are_published_through_actual_reactive_command() {
    let _environment = Environment::new();
    let service = DockBusyService::new(None, None);
    let errors = Rc::new(RefCell::new(Vec::new()));
    let seen = errors.clone();
    let _subscription = service
        .reload_command
        .get()
        .unwrap()
        .thrown_exceptions()
        .subscribe_next(move |error| seen.borrow_mut().push(error));
    let error: ObservableError = Rc::new(TaskWorkError {
        message: "reload error".into(),
    });
    let captured = error.clone();
    service.set_reload_handler(Some(Rc::new(move || {
        Task::from_exception(captured.clone())
    })));
    service.reload_command().execute(None);
    assert!(Rc::ptr_eq(&errors.borrow()[0], &error));
    service.set_reload_handler(Some(Rc::new(|| panic!("reload panic"))));
    service.reload_command().execute(None);
    assert_eq!(errors.borrow()[1].to_string(), "reload panic");
    assert!(service.reload_command().can_execute(None));
}

#[test]
fn scopes_and_pending_runs_retain_service_until_explicit_end_without_default_drop_disposal() {
    let _environment = Environment::new();
    let service = DockBusyService::new(None, None);
    let weak = Rc::downgrade(&service);
    let scope = service.begin("held".into());
    drop(service);
    assert!(weak.upgrade().is_some());
    scope.dispose();
    assert!(weak.upgrade().is_none());
    let service = DockBusyService::new(None, None);
    drop(service.begin("dropped handle".into()));
    assert!(service.is_busy());
    let service = DockBusyService::new(None, None);
    let weak = Rc::downgrade(&service);
    let work = TaskCompletionSource::new();
    let pending = work.task();
    let task = service.run_async("held task".into(), Rc::new(move || pending.clone()));
    drop(service);
    assert!(weak.upgrade().is_some());
    work.set_result(());
    assert!(task.is_completed_successfully());
    assert!(weak.upgrade().is_none());
}

struct GlobalProbe {
    reactive: ReactiveObject,
    begins: Rc<Cell<usize>>,
    ends: Rc<Cell<usize>>,
    throw_begin: Cell<bool>,
    throw_end: Cell<bool>,
}
reactive_object!(GlobalProbe);
impl GlobalProbe {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            begins: Rc::new(Cell::new(0)),
            ends: Rc::new(Cell::new(0)),
            throw_begin: Cell::new(false),
            throw_end: Cell::new(false),
        })
    }
}
impl IDockGlobalBusyService for GlobalProbe {
    fn is_busy(&self) -> bool {
        self.begins.get() > self.ends.get()
    }
    fn message(&self) -> Option<String> {
        None
    }
    fn begin(&self, _message: Option<String>) -> Rc<dyn IDisposable> {
        self.begins.set(self.begins.get() + 1);
        assert!(!self.throw_begin.get(), "global begin failed");
        let ends = self.ends.clone();
        let fail = self.throw_end.get();
        ActionDisposable::new(move || {
            ends.set(ends.get() + 1);
            assert!(!fail, "global dispose failed");
        })
    }
}

#[test]
fn run_async_captures_begin_failure_and_cleanup_failure_supersedes_work_failure() {
    let _environment = Environment::new();
    let global = GlobalProbe::new();
    let service = DockBusyService::new(Some(global.clone()), None);
    global.throw_begin.set(true);
    let called = Rc::new(Cell::new(false));
    let seen = called.clone();
    let task = service.run_async(
        "not started".into(),
        Rc::new(move || {
            seen.set(true);
            Task::from_result(())
        }),
    );
    assert!(task.is_faulted());
    assert_eq!(task.exception().unwrap().to_string(), "global begin failed");
    assert!(!called.get());
    assert!(!service.is_busy());
    assert_eq!(global.ends.get(), 0);
    global.throw_begin.set(false);
    global.throw_end.set(true);
    let task = service.run_async(
        "cleanup failure".into(),
        Rc::new(|| {
            Task::from_exception(Rc::new(TaskWorkError {
                message: "original work failure".into(),
            }))
        }),
    );
    assert!(task.is_faulted());
    assert_eq!(
        task.exception().unwrap().to_string(),
        "global dispose failed"
    );
    assert!(!service.is_busy());
    assert_eq!(global.ends.get(), 1);
    global.throw_end.set(false);
    let task = service.run_async("normal".into(), Rc::new(|| Task::from_result(())));
    assert!(task.is_completed_successfully());
    assert_eq!(global.ends.get(), 2);
}

#[test]
fn synchronous_local_cleanup_notification_failure_prevents_later_global_disposal_like_source() {
    let _environment = Environment::new();
    let global = GlobalProbe::new();
    let service = DockBusyService::new(Some(global.clone()), None);
    let weak = Rc::downgrade(&service);
    service.property_changed().add(Rc::new(move |name| {
        if name == "IsBusy" && !weak.upgrade().unwrap().is_busy() {
            panic!("local notification failed");
        }
    }));
    let task = service.run_async("scope".into(), Rc::new(|| Task::from_result(())));
    assert!(task.is_faulted());
    assert_eq!(
        task.exception().unwrap().to_string(),
        "local notification failed"
    );
    assert!(!service.is_busy());
    assert_eq!(service.message().as_deref(), Some("scope"));
    assert_eq!(global.ends.get(), 0);
    assert!(global.is_busy());
}

#[test]
fn same_and_foreign_synchronization_contexts_cover_both_handler_dispatch_paths() {
    use ferroui_system::threading::set_synchronization_context;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Default)]
    struct Context {
        posts: AtomicUsize,
    }
    impl SynchronizationContext for Context {
        fn post(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn send(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn post_local(&self, work: Box<dyn FnOnce()>) {
            self.posts.fetch_add(1, Ordering::Relaxed);
            work();
        }
        fn send_local(&self, work: Box<dyn FnOnce()>) {
            work();
        }
    }
    let _environment = Environment::new();
    let context = Arc::new(Context::default());
    set_synchronization_context(Some(context.clone()));
    let service = DockBusyService::new(None, None);
    service.set_reload_handler(Some(Rc::new(|| Task::from_result(()))));
    assert!(service.can_reload());
    service.begin("inline".into()).dispose();
    assert_eq!(context.posts.load(Ordering::Relaxed), 0);
    set_synchronization_context(None);
    service.set_reload_handler(None);
    service.begin("posted".into()).dispose();
    service.update_message(Some("posted message".into()));
    assert!(!service.can_reload());
    assert_eq!(context.posts.load(Ordering::Relaxed), 4);
    assert_eq!(service.message().as_deref(), Some("posted message"));
}
