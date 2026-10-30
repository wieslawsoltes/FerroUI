use super::*;
use crate::test_environment::Environment;
use ferrodock_model::services::*;
use ferrodock_model::BoxedValue;
use ferroui_reactive::concurrency::HistoricalScheduler;
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::collections::specialized::NotifyCollectionChangedAction as Action;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::threading::tasks::{block_on, Task};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

struct Counter(Cell<usize>);
impl IDisposable for Counter {
    fn dispose(&self) {
        self.0.set(self.0.get() + 1);
    }
}
macro_rules! stacks {
    ($module:ident,$service:ident,$global:ident,$list:ident,$active:ident,$has:ident,$open:ident,$active_name:literal,$has_name:literal,$result:ty,$answer:expr,$foreign:expr,|$s:ident| $start:expr) => {
        mod $module {
            use super::*;
            fn start($s:&Rc<$service>)->Task<$result> { $start }
            #[test]
            fn live_collection_defaults_stack_selection_and_cancel_snapshot() {
                let _environment=Environment::new(); let global=$global::new(None,None); let service=$service::new(Some(global.clone()),None);
                let view=service.$list(); assert!(view.ptr_eq(&service.$list())); assert!(view.is_empty()); assert!(service.$active().is_none()); assert!(!service.$has());
                let first=start(&service); let first_request=view.get(0); let second=start(&service);
                assert_eq!(view.count(),2); assert!(Rc::ptr_eq(&service.$active().unwrap(),&view.get(1))); assert!(global.$open());
                service.close(first_request,$answer); assert!(first.is_completed_successfully()); assert_eq!(view.count(),1); assert!(!second.is_completed());
                service.cancel_all(); assert!(second.is_completed_successfully()); assert!(view.is_empty()); assert!(service.$active().is_none()); assert!(!global.$open());
                service.cancel_all();
            }
            #[test]
            fn notifications_remove_then_dispose_then_complete_before_active_reset() {
                let _environment=Environment::new(); let global=$global::new(None,None); let service=$service::new(Some(global.clone()),None); let view=service.$list();
                let events=Rc::new(RefCell::new(Vec::new())); let seen=events.clone(); view.property_changed().add(Rc::new(move |name| seen.borrow_mut().push(name.to_owned())));
                let seen=events.clone(); view.add_collection_changed(Rc::new(move |args| seen.borrow_mut().push(format!("{:?}",args.action))));
                let seen=events.clone(); service.property_changed().add(Rc::new(move |name| seen.borrow_mut().push(name.to_owned())));
                let task=start(&service); assert_eq!(&*events.borrow(),&["Count","Item[]","Add",$active_name,$has_name]); events.borrow_mut().clear();
                let request=view.get(0); let weak=Rc::downgrade(&service); let seen=events.clone(); let global_probe=global.clone();
                task.continue_with(move |_| { let service=weak.upgrade().unwrap(); assert!(!service.$has()); assert!(!global_probe.$open()); assert!(service.$active().is_some()); seen.borrow_mut().push("Complete".into()); });
                service.close(request,$answer);
                assert_eq!(&*events.borrow(),&["Count","Item[]","Remove","Complete",$active_name,$has_name]);
            }
            #[test]
            fn queued_insertion_close_and_asynchronous_await_complete_through_system_executor() {
                let _environment=Environment::new(); let scheduler=HistoricalScheduler::historical(); RxSchedulers::set_main_thread_scheduler(scheduler.clone());
                let service=$service::new(None,None); let task=start(&service); assert!(!service.$has()); assert!(!task.is_completed());
                scheduler.start(); assert!(service.$has()); assert!(!task.is_completed()); service.close(service.$active().unwrap(),$answer); assert!(service.$has());
                scheduler.start(); assert!(!service.$has()); block_on(task).unwrap();
            }
            #[test]
            fn foreign_and_already_closed_requests_still_dispose_and_complete_without_stack_notifications() {
                let _environment=Environment::new(); let service=$service::new(None,None); let request=$foreign;
                let disposed=Rc::new(Counter(Cell::new(0))); request.set_global_handle(Some(disposed.clone()));
                let changes=Rc::new(Cell::new(0)); let seen=changes.clone(); service.property_changed().add(Rc::new(move |_| seen.set(seen.get()+1)));
                service.close(request.clone(),$answer); service.close(request.clone(),$answer);
                assert_eq!(disposed.0.get(),2); assert!(request.task().is_completed_successfully()); assert_eq!(changes.get(),0);
            }
            #[test]
            fn cancel_all_snapshot_keeps_a_request_opened_reentrantly() {
                let _environment=Environment::new(); let service=$service::new(None,None); let first=start(&service); let second=start(&service); let pending=Rc::new(RefCell::new(None)); let seen=pending.clone(); let weak=Rc::downgrade(&service);
                first.continue_with(move |_| { seen.replace(Some(start(&weak.upgrade().unwrap()))); });
                service.cancel_all(); assert!(first.is_completed()); assert!(second.is_completed()); assert_eq!(service.$list().count(),1); assert!(!pending.borrow().as_ref().unwrap().is_completed());
                service.cancel_all(); assert!(pending.borrow().as_ref().unwrap().is_completed());
            }
            #[test]
            fn reentrant_close_during_add_preserves_source_stale_active_assignment() {
                let _environment=Environment::new(); let service=$service::new(None,None); let weak=Rc::downgrade(&service);
                service.$list().add_collection_changed(Rc::new(move |args| { if args.action==Action::Add { weak.upgrade().unwrap().close(args.new_items[0].clone(),$answer); } }));
                let task=start(&service); assert!(task.is_completed_successfully()); assert!(!service.$has()); assert!(service.$active().is_some());
            }
            #[test]
            fn add_notification_failure_faults_task_without_invented_rollback() {
                let _environment=Environment::new(); let global=$global::new(None,None); let service=$service::new(Some(global.clone()),None);
                let view=service.$list(); let token=view.add_collection_changed(Rc::new(|args| { if args.action==Action::Add { panic!("add listener failed"); } }));
                let task=start(&service); assert!(task.is_faulted()); assert_eq!(task.exception().unwrap().to_string(),"add listener failed"); assert!(service.$has()); assert!(service.$active().is_none()); assert!(global.$open());
                view.remove_collection_changed(token); service.cancel_all(); assert!(!global.$open());
            }
            #[test]
            fn throwing_global_disposal_leaves_source_removal_and_completion_order() {
                struct Throwing;
                impl IDisposable for Throwing { fn dispose(&self) { panic!("dispose failed"); } }
                let _environment=Environment::new(); let service=$service::new(None,None); let task=start(&service); let request=service.$list().get(0); request.set_global_handle(Some(Rc::new(Throwing)));
                assert!(catch_unwind(AssertUnwindSafe(|| service.close(request.clone(),$answer))).is_err()); assert!(!service.$has()); assert!(!task.is_completed()); assert!(service.$active().is_some());
                request.set_global_handle(None); service.close(request,$answer); assert!(task.is_completed()); assert!(service.$active().is_some());
            }
            #[test]
            fn pending_request_retains_service_until_closed_and_releases_after_completion() {
                let _environment=Environment::new(); let service=$service::new(None,None); let task=start(&service); let request=service.$active().unwrap(); let weak=Rc::downgrade(&service); drop(service); assert!(weak.upgrade().is_some());
                let target=weak.upgrade().unwrap(); target.close(request,$answer); drop(target); assert!(task.is_completed()); assert!(weak.upgrade().is_none());
            }
        }
    };
}
stacks!(
    confirmations,
    DockConfirmationService,
    DockGlobalConfirmationService,
    confirmations,
    active_confirmation,
    has_confirmations,
    is_confirmation_open,
    "ActiveConfirmation",
    "HasConfirmations",
    bool,
    true,
    ConfirmationRequest::new(
        "foreign".into(),
        "message".into(),
        "ok".into(),
        "cancel".into(),
        Rc::new(|_, _| {})
    ),
    |service| service.confirm_async_default("title".into(), "message".into())
);
stacks!(
    dialogs,
    DockDialogService,
    DockGlobalDialogService,
    dialogs,
    active_dialog,
    has_dialogs,
    is_dialog_open,
    "ActiveDialog",
    "HasDialogs",
    Option<BoxedValue>,
    None,
    DialogRequest::new(Rc::new("foreign"), None, Rc::new(|_, _| {})),
    |service| service.show_async_boxed(DialogContent::new(Rc::new("content")), None)
);

#[test]
fn confirmation_default_labels_explicit_labels_and_command_results_use_erased_contract() {
    let _environment = Environment::new();
    let service: Rc<dyn IDockConfirmationService> = DockConfirmationService::new(None, None);
    let task = service.confirm_async_default("title".into(), "message".into());
    let request = service.active_confirmation().unwrap();
    assert_eq!(
        (
            request.title(),
            request.message(),
            request.confirm_text(),
            request.cancel_text()
        ),
        ("title", "message", "Confirm", "Cancel")
    );
    request.confirm().execute(None);
    assert!(task.result().unwrap());
    let task = service.confirm_async("other".into(), "message".into(), "yes".into(), "no".into());
    let request = service.active_confirmation().unwrap();
    assert_eq!(
        (request.confirm_text(), request.cancel_text()),
        ("yes", "no")
    );
    request.cancel().execute(None);
    assert!(!task.result().unwrap());
}
struct SelfClosing {
    action: RefCell<Option<DialogContentCloseAction>>,
    immediate: bool,
}
impl IDockDialogContent for SelfClosing {
    fn set_close_action(&self, action: DialogContentCloseAction) {
        self.action.replace(Some(action.clone()));
        if self.immediate {
            action(Some(Rc::new(42_i32)));
        }
    }
}
#[test]
fn typed_dialog_content_can_close_before_insertion_and_keeps_original_content_identity() {
    let _environment = Environment::new();
    let global = DockGlobalDialogService::new(None, None);
    let service = DockDialogService::new(Some(global.clone()), None);
    let content = Rc::new(SelfClosing {
        action: RefCell::new(None),
        immediate: true,
    });
    let task = service.show_async::<i32>(
        DialogContent::from_dialog_content(content.clone()),
        Some("title".into()),
    );
    assert_eq!(task.result().unwrap(), 42);
    assert!(!service.has_dialogs());
    assert!(!global.is_dialog_open());
    let content = Rc::new(SelfClosing {
        action: RefCell::new(None),
        immediate: false,
    });
    let weak = Rc::downgrade(&content);
    let task = service.show_async::<i32>(DialogContent::from_dialog_content(content.clone()), None);
    let request = service.active_dialog().unwrap();
    assert!(Rc::ptr_eq(
        &request.content(),
        &(content.clone() as BoxedValue)
    ));
    content.action.borrow().as_ref().unwrap()(Some(Rc::new(7_i32)));
    assert_eq!(task.result().unwrap(), 7);
    drop(request);
    drop(content);
    assert!(weak.upgrade().is_none());
}
#[test]
fn queued_content_close_precedes_add_and_uses_completed_request_guard() {
    let _environment = Environment::new();
    let scheduler = HistoricalScheduler::historical();
    RxSchedulers::set_main_thread_scheduler(scheduler.clone());
    let service = DockDialogService::new(None, None);
    let content = Rc::new(SelfClosing {
        action: RefCell::new(None),
        immediate: true,
    });
    let task = service.show_async::<i32>(DialogContent::from_dialog_content(content), None);
    scheduler.start();
    assert!(!service.has_dialogs());
    assert!(service.active_dialog().is_none());
    assert_eq!(block_on(task).unwrap(), 42);
}
#[test]
fn generic_dialog_results_through_erased_service_use_source_defaults_and_reference_identity() {
    let _environment = Environment::new();
    let service: Rc<dyn IDockDialogService> = DockDialogService::new(None, None);
    let content: BoxedValue = Rc::new("content");
    let task = service.show_async::<i32>(content.clone(), None);
    service.close(service.active_dialog().unwrap(), Some(Rc::new(7_i64)));
    assert_eq!(task.result().unwrap(), 0);
    let task = service.show_async::<Option<i32>>(content.clone(), None);
    service.close(service.active_dialog().unwrap(), None);
    assert_eq!(task.result().unwrap(), None);
    let value = Rc::new("reference".to_owned());
    let task = service.show_async::<Option<Rc<String>>>(content, None);
    service.close(service.active_dialog().unwrap(), Some(value.clone()));
    assert!(Rc::ptr_eq(&task.result().unwrap().unwrap(), &value));
}
#[test]
fn host_adapter_retains_all_six_actual_service_instances() {
    use crate::overlays::hosting::HostOverlayServicesAdapter;
    let _environment = Environment::new();
    let gb: Rc<dyn IDockGlobalBusyService> = DockGlobalBusyService::new(None, None);
    let gd: Rc<dyn IDockGlobalDialogService> = DockGlobalDialogService::new(None, None);
    let gc: Rc<dyn IDockGlobalConfirmationService> = DockGlobalConfirmationService::new(None, None);
    let busy: Rc<dyn IDockBusyService> = DockBusyService::new(Some(gb.clone()), None);
    let dialogs: Rc<dyn IDockDialogService> = DockDialogService::new(Some(gd.clone()), None);
    let confirmations: Rc<dyn IDockConfirmationService> =
        DockConfirmationService::new(Some(gc.clone()), None);
    let host: Rc<dyn IHostOverlayServices> = HostOverlayServicesAdapter::new(
        busy.clone(),
        dialogs.clone(),
        confirmations.clone(),
        gb.clone(),
        gd.clone(),
        gc.clone(),
    );
    assert!(Rc::ptr_eq(&host.busy(), &busy));
    assert!(Rc::ptr_eq(&host.dialogs(), &dialogs));
    assert!(Rc::ptr_eq(&host.confirmations(), &confirmations));
    assert!(Rc::ptr_eq(&host.global_busy_service(), &gb));
    assert!(Rc::ptr_eq(&host.global_dialog_service(), &gd));
    assert!(Rc::ptr_eq(&host.global_confirmation_service(), &gc));
}

#[test]
fn same_context_closes_and_cancels_directly_and_posting_failure_faults_show_task() {
    use ferroui_system::threading::{set_synchronization_context, SynchronizationContext};
    use std::sync::Arc;
    struct Context;
    impl SynchronizationContext for Context {
        fn post(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn send(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn post_local(&self, work: Box<dyn FnOnce()>) {
            work();
        }
        fn send_local(&self, work: Box<dyn FnOnce()>) {
            work();
        }
    }
    let _environment = Environment::new();
    set_synchronization_context(Some(Arc::new(Context)));
    let confirmations = DockConfirmationService::new(None, None);
    let task = confirmations.confirm_async_default("t".into(), "m".into());
    confirmations.cancel_all();
    assert!(!task.result().unwrap());
    let dialogs = DockDialogService::new(None, None);
    let task = dialogs.show_async::<i32>(DialogContent::new(Rc::new("content")), None);
    dialogs.cancel_all();
    assert_eq!(task.result().unwrap(), 0);
    struct Throwing;
    impl SynchronizationContext for Throwing {
        fn post(&self, _work: Box<dyn FnOnce() + Send>) {
            panic!("post failed");
        }
        fn send(&self, _work: Box<dyn FnOnce() + Send>) {
            panic!("send failed");
        }
        fn post_local(&self, _work: Box<dyn FnOnce()>) {
            panic!("post failed");
        }
        fn send_local(&self, _work: Box<dyn FnOnce()>) {
            panic!("send failed");
        }
    }
    let dialogs = DockDialogService::new(None, Some(Arc::new(Throwing)));
    let task = dialogs.show_async::<i32>(DialogContent::new(Rc::new("content")), None);
    assert!(task.is_faulted());
    assert_eq!(task.exception().unwrap().to_string(), "post failed");
    assert!(!dialogs.has_dialogs());
}
