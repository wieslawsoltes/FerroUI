use super::tests::{run, update, window};
use super::*;
use ferrodock_model::services::*;
use ferrodock_model_reactiveui_services::overlays::services::*;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use ferroui_controls::{Border, Control};
use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::{ICommand, RxSchedulers};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::threading::{
    current_synchronization_context, set_synchronization_context, tasks::Task,
    SynchronizationContext,
};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

struct Environment(Option<Arc<dyn SynchronizationContext>>, Rc<dyn IScheduler>);
impl Environment {
    fn new() -> Self {
        let old = Self(
            current_synchronization_context(),
            RxSchedulers::main_thread_scheduler(),
        );
        set_synchronization_context(None);
        RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
        old
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        set_synchronization_context(self.0.take());
        RxSchedulers::set_main_thread_scheduler(self.1.clone());
    }
}
fn service_test(test: impl FnOnce() + Send + 'static) {
    run(move || {
        let _environment = Environment::new();
        test();
    });
}

#[test]
fn source_defaults_and_read_only_direct_properties_are_retained() {
    run(|| {
        let busy = BusyOverlayControl::new();
        assert!(busy.busy_service().is_none());
        assert!(busy.global_busy_service().is_none());
        assert!(busy.content().is_none());
        assert!(busy.content_template().is_none());
        assert!(busy.blocks_input());
        assert!(busy.is_content_enabled());
        assert!(busy.is_reload_visible());
        assert!(!busy.is_reload_enabled());
        assert!(!busy.is_busy());
        assert!(!busy.show_progress());
        assert!(busy.busy_message().is_none());
        assert!(busy.reload_command().is_none());
        assert_eq!(busy.show_reload_button(), None);
        assert!(BusyOverlayControl::is_busy_property().is_read_only());
        assert!(BusyOverlayControl::reload_command_property().is_read_only());
        busy.on_attached_to_visual_tree();
        assert!(!busy.is_reload_visible());
        let dialog = DialogOverlayControl::new();
        assert!(dialog.is_content_enabled());
        assert!(dialog.blocks_input());
        assert!(!dialog.is_overlay_visible());
        assert!(!dialog.show_dialogs());
        assert!(!dialog.show_global_message());
        assert!(dialog.overlay_message().is_none());
        assert!(dialog.dialogs().is_none());
        assert!(DialogOverlayControl::dialogs_property().is_read_only());
        let confirmation = ConfirmationOverlayControl::new();
        assert!(confirmation.is_content_enabled());
        assert!(confirmation.blocks_input());
        assert!(!confirmation.is_overlay_visible());
        assert!(!confirmation.show_confirmations());
        assert!(!confirmation.show_global_message());
        assert!(confirmation.overlay_message().is_none());
        assert!(confirmation.confirmations().is_none());
        assert!(ConfirmationOverlayControl::confirmations_property().is_read_only());
    });
}
#[test]
fn busy_combines_local_global_scopes_progress_messages_and_input() {
    service_test(|| {
        let local = DockBusyService::new(None, None);
        let global = DockGlobalBusyService::new(None, None);
        let control = BusyOverlayControl::new();
        control.set_busy_service(Some(local.clone()));
        control.set_global_busy_service(Some(global.clone()));
        let global_scope = global.begin(Some("global".into()));
        assert!(control.is_busy());
        assert!(!control.show_progress());
        assert!(!control.is_content_enabled());
        assert_eq!(control.busy_message().as_deref(), Some("global"));
        let local_scope = local.begin("local".into());
        assert!(control.show_progress());
        assert_eq!(control.busy_message().as_deref(), Some("local"));
        control.set_blocks_input(false);
        assert!(control.is_content_enabled());
        local_scope.dispose();
        assert!(!control.show_progress());
        assert_eq!(control.busy_message().as_deref(), Some("global"));
        global_scope.dispose();
        assert!(!control.is_busy());
        assert!(control.busy_message().is_none());
    });
}
#[test]
fn reload_visibility_override_handler_enablement_and_identity_use_original_command() {
    service_test(|| {
        let service = DockBusyService::new(None, None);
        let control = BusyOverlayControl::new();
        control.set_busy_service(Some(service.clone()));
        let command = service.reload_command();
        assert!(Rc::ptr_eq(&command, &control.reload_command().unwrap()));
        assert!(control.is_reload_visible());
        assert!(!control.is_reload_enabled());
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        let _subscription = control.property_changed(move |e| {
            if e.property() == BusyOverlayControl::reload_command_property().as_property() {
                observed.set(observed.get() + 1);
            }
        });
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        service.set_reload_handler(Some(Rc::new(move || {
            called.set(called.get() + 1);
            Task::from_result(())
        })));
        assert!(control.is_reload_enabled());
        let boxed: BoxedValue = Rc::new(command.clone());
        let adapted = ValueTypes::try_convert(
            Some(&boxed),
            ValueType::of::<Rc<dyn ferroui_base::input::ICommand>>(),
        )
        .unwrap();
        let adapted = adapted.unwrap();
        let adapted = adapted
            .downcast_ref::<Rc<dyn ferroui_base::input::ICommand>>()
            .unwrap();
        assert!(adapted.can_execute(None));
        adapted.execute(None);
        assert_eq!(calls.get(), 1);
        service.set_is_reload_visible(false);
        assert!(!control.is_reload_visible());
        control.set_show_reload_button(Some(true));
        assert!(control.is_reload_visible());
        control.set_show_reload_button(Some(false));
        service.set_is_reload_visible(true);
        assert!(!control.is_reload_visible());
        control.set_show_reload_button(None);
        assert!(control.is_reload_visible());
        let scope = service.begin("work".into());
        assert!(!control.is_reload_enabled());
        scope.dispose();
        assert!(control.is_reload_enabled());
        assert_eq!(changes.get(), 0);
        assert!(Rc::ptr_eq(&command, &control.reload_command().unwrap()));
    });
}
#[test]
fn busy_subscribes_before_attachment_replaces_once_and_detaches_without_reset() {
    service_test(|| {
        let first = DockBusyService::new(None, None);
        let second = DockBusyService::new(None, None);
        let global = DockGlobalBusyService::new(None, None);
        let control = BusyOverlayControl::new();
        let first_count = first.property_changed().handler_count();
        control.set_busy_service(Some(first.clone()));
        assert_eq!(first.property_changed().handler_count(), first_count + 1);
        control.set_busy_service(Some(first.clone()));
        control.on_attached_to_visual_tree();
        assert_eq!(first.property_changed().handler_count(), first_count + 1);
        control.set_busy_service(Some(second.clone()));
        assert_eq!(first.property_changed().handler_count(), first_count);
        control.set_global_busy_service(Some(global.clone()));
        let scope = second.begin("retain".into());
        let command = control.reload_command().unwrap();
        control.on_detached_from_visual_tree();
        assert!(control.is_busy());
        assert_eq!(control.busy_message().as_deref(), Some("retain"));
        assert!(Rc::ptr_eq(&command, &control.reload_command().unwrap()));
        scope.dispose();
        let global_scope = global.begin(Some("after detach".into()));
        assert!(control.show_progress());
        assert_eq!(control.busy_message().as_deref(), Some("retain"));
        control.on_attached_to_visual_tree();
        assert!(control.is_busy());
        assert!(!control.show_progress());
        assert_eq!(control.busy_message().as_deref(), Some("after detach"));
        global_scope.dispose();
    });
}
#[test]
fn dialog_requests_stay_live_and_local_requests_take_priority_over_global_message() {
    service_test(|| {
        let local = DockDialogService::new(None, None);
        let global = DockGlobalDialogService::new(None, None);
        let control = DialogOverlayControl::new();
        control.set_dialog_service(Some(local.clone()));
        control.set_global_dialog_service(Some(global.clone()));
        let collection = control.dialogs().unwrap();
        assert!(collection.ptr_eq(&local.dialogs()));
        let scope = global.begin(Some("elsewhere".into()));
        assert!(control.is_overlay_visible());
        assert!(control.show_global_message());
        assert!(!control.show_dialogs());
        assert_eq!(control.overlay_message().as_deref(), Some("elsewhere"));
        let _task = local.show_async_boxed(
            DialogContent::new(Rc::new("content".to_owned())),
            Some("title".into()),
        );
        assert_eq!(collection.count(), 1);
        assert!(control.show_dialogs());
        assert!(!control.show_global_message());
        assert!(control.overlay_message().is_none());
        assert!(!control.is_content_enabled());
        control.set_blocks_input(false);
        assert!(control.is_content_enabled());
        local.close(collection.get(0), None);
        assert!(collection.is_empty());
        assert!(!control.show_dialogs());
        assert!(control.show_global_message());
        scope.dispose();
        assert!(!control.is_overlay_visible());
    });
}
#[test]
fn confirmation_requests_stay_live_and_local_requests_take_priority_over_global_message() {
    service_test(|| {
        let local = DockConfirmationService::new(None, None);
        let global = DockGlobalConfirmationService::new(None, None);
        let control = ConfirmationOverlayControl::new();
        control.set_confirmation_service(Some(local.clone()));
        control.set_global_confirmation_service(Some(global.clone()));
        let collection = control.confirmations().unwrap();
        assert!(collection.ptr_eq(&local.confirmations()));
        let scope = global.begin(Some("elsewhere".into()));
        assert!(control.is_overlay_visible());
        assert!(control.show_global_message());
        assert!(!control.show_confirmations());
        assert_eq!(control.overlay_message().as_deref(), Some("elsewhere"));
        let _task = local.confirm_async_default("title".into(), "message".into());
        assert_eq!(collection.count(), 1);
        assert!(control.show_confirmations());
        assert!(!control.show_global_message());
        assert!(control.overlay_message().is_none());
        assert!(!control.is_content_enabled());
        control.set_blocks_input(false);
        assert!(control.is_content_enabled());
        local.close(collection.get(0), true);
        assert!(collection.is_empty());
        assert!(!control.show_confirmations());
        assert!(control.show_global_message());
        scope.dispose();
        assert!(!control.is_overlay_visible());
    });
}
#[test]
fn visual_tree_dialog_detach_clears_only_requests_and_reattach_restores_current_state() {
    service_test(|| {
        let service = DockDialogService::new(None, None);
        let control = DialogOverlayControl::new();
        control.set_dialog_service(Some(service.clone()));
        let _task = service.show_async_boxed(DialogContent::new(Rc::new(4)), None);
        let w = window(control.clone().upcast());
        assert!(control.dialogs().is_some());
        w.set_content(None);
        update(&w);
        assert!(control.dialogs().is_none());
        assert!(control.show_dialogs());
        assert!(!control.is_content_enabled());
        service.cancel_all();
        assert!(control.show_dialogs());
        w.set_content(Some(Control::boxed(control.clone())));
        update(&w);
        assert!(!control.show_dialogs());
        assert!(control.is_content_enabled());
        assert!(control.dialogs().unwrap().ptr_eq(&service.dialogs()));
        w.close();
    });
}
#[test]
fn visual_tree_confirmation_detach_clears_only_requests_and_reattach_restores_current_state() {
    service_test(|| {
        let service = DockConfirmationService::new(None, None);
        let control = ConfirmationOverlayControl::new();
        control.set_confirmation_service(Some(service.clone()));
        let _task = service.confirm_async_default("title".into(), "message".into());
        let w = window(control.clone().upcast());
        w.set_content(None);
        update(&w);
        assert!(control.confirmations().is_none());
        assert!(control.show_confirmations());
        assert!(!control.is_content_enabled());
        service.cancel_all();
        assert!(control.show_confirmations());
        w.set_content(Some(Control::boxed(control.clone())));
        update(&w);
        assert!(!control.show_confirmations());
        assert!(control.is_content_enabled());
        assert!(control
            .confirmations()
            .unwrap()
            .ptr_eq(&service.confirmations()));
        w.close();
    });
}
#[test]
fn actual_service_controls_participate_in_host_content_pipeline_and_native_contract_casts() {
    run(|| {
        let content = Border::new();
        let busy = BusyOverlayControl::new();
        let dialog = DialogOverlayControl::new();
        let confirmation = ConfirmationOverlayControl::new();
        let layers = OverlayLayerCollection::new();
        for (z, control) in [
            (30, busy.clone().upcast::<Control>()),
            (10, dialog.clone().upcast()),
            (20, confirmation.clone().upcast()),
        ] {
            let layer = OverlayLayer::new();
            layer.set_z_index(z);
            layer.set_overlay(Some(control));
            layer.set_blocks_input(false);
            layers.add(Some(OverlayLayer::as_layer(layer)));
        }
        let host = OverlayHost::new();
        host.set_use_service_layers(false);
        host.set_content(Some(Control::boxed(content.clone())));
        host.set_overlay_layers(Some(layers));
        let w = window(host.clone().upcast());
        assert_eq!(host.child(), Some(busy.clone().upcast::<Control>()));
        assert_eq!(
            Control::from_boxed(&busy.content().unwrap()),
            Some(confirmation.clone().upcast::<Control>())
        );
        assert_eq!(
            Control::from_boxed(&confirmation.content().unwrap()),
            Some(dialog.clone().upcast::<Control>())
        );
        let presenter = Control::from_boxed(&dialog.content().unwrap())
            .unwrap()
            .cast::<ferroui_controls::presenters::ContentPresenter>()
            .unwrap();
        assert_eq!(
            Control::from_boxed(&presenter.content().unwrap()),
            Some(content.upcast())
        );
        assert!(!busy.blocks_input());
        assert!(!dialog.blocks_input());
        assert!(!confirmation.blocks_input());
        let first = BusyOverlayControl::as_visual_tree_lifecycle(busy.clone());
        let second = BusyOverlayControl::as_visual_tree_lifecycle(busy.clone());
        assert!(Rc::ptr_eq(&first, &second));
        let projected = ValueTypes::try_cast(
            &Control::boxed(busy),
            ValueType::of::<Rc<dyn IVisualTreeLifecycle>>(),
        )
        .unwrap();
        assert!(Rc::ptr_eq(
            &first,
            projected
                .downcast_ref::<Rc<dyn IVisualTreeLifecycle>>()
                .unwrap()
        ));
        first.on_detached_from_visual_tree();
        first.on_attached_to_visual_tree();
        w.close();
    });
}
#[test]
fn busy_reentrant_direct_notifications_reread_services_in_source_order() {
    service_test(|| {
        let service = DockBusyService::new(None, None);
        let control = BusyOverlayControl::new();
        control.set_busy_service(Some(service.clone()));
        let weak = control.downgrade();
        let once = Cell::new(false);
        let _subscription = control.property_changed(move |e| {
            if e.property() == BusyOverlayControl::is_busy_property().as_property()
                && !once.replace(true)
            {
                weak.upgrade().unwrap().set_busy_service(None);
            }
        });
        let scope = service.begin("changed".into());
        // Captured localBusy remains true in the outer sync, while its later
        // reads see the service removed by the nested direct notification.
        assert!(!control.is_busy());
        assert!(control.show_progress());
        assert!(control.busy_message().is_none());
        assert!(control.reload_command().is_none());
        assert!(!control.is_reload_enabled());
        scope.dispose();
    });
}

struct BusyProbe {
    changed: Event<str>,
    busy: Cell<bool>,
    trace: RefCell<Vec<&'static str>>,
    delegate: Rc<DockBusyService>,
}
impl BusyProbe {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            changed: Event::new(),
            busy: Cell::new(false),
            trace: RefCell::new(Vec::new()),
            delegate: DockBusyService::new(None, None),
        })
    }
    fn record(&self, name: &'static str) {
        self.trace.borrow_mut().push(name);
    }
}
impl INotifyPropertyChanged for BusyProbe {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
impl IDockBusyService for BusyProbe {
    fn is_busy(&self) -> bool {
        self.record("IsBusy");
        self.busy.get()
    }
    fn message(&self) -> Option<String> {
        self.record("Message");
        Some("probe".into())
    }
    fn is_reload_visible(&self) -> bool {
        self.record("IsReloadVisible");
        true
    }
    fn set_is_reload_visible(&self, value: bool) {
        self.delegate.set_is_reload_visible(value);
    }
    fn can_reload(&self) -> bool {
        self.record("CanReload");
        true
    }
    fn reload_command(&self) -> Rc<dyn ICommand> {
        self.record("ReloadCommand");
        self.delegate.reload_command()
    }
    fn begin(&self, message: String) -> Rc<dyn IDisposable> {
        self.delegate.begin(message)
    }
    fn run_async(&self, message: String, action: BusyAction) -> Task<()> {
        self.delegate.run_async(message, action)
    }
    fn update_message(&self, message: Option<String>) {
        self.delegate.update_message(message);
    }
    fn set_reload_handler(&self, handler: Option<BusyAction>) {
        self.delegate.set_reload_handler(handler);
    }
}
#[test]
fn busy_filters_exact_property_names_and_preserves_repeated_getters_and_short_circuits() {
    service_test(|| {
        let probe = BusyProbe::new();
        let control = BusyOverlayControl::new();
        control.set_busy_service(Some(probe.clone()));
        assert_eq!(
            &*probe.trace.borrow(),
            &[
                "IsBusy",
                "ReloadCommand",
                "IsReloadVisible",
                "ReloadCommand",
                "CanReload"
            ]
        );
        probe.trace.borrow_mut().clear();
        probe.busy.set(true);
        for name in ["", "Other", "ReloadCommand"] {
            probe.changed.raise(name);
        }
        assert!(probe.trace.borrow().is_empty());
        assert!(!control.is_busy());
        probe.changed.raise("IsBusy");
        assert!(control.is_busy());
        assert_eq!(
            &*probe.trace.borrow(),
            &[
                "IsBusy",
                "ReloadCommand",
                "IsReloadVisible",
                "Message",
                "ReloadCommand"
            ]
        );
        probe.trace.borrow_mut().clear();
        control.set_show_reload_button(Some(false));
        assert_eq!(
            &*probe.trace.borrow(),
            &["IsBusy", "ReloadCommand", "Message", "ReloadCommand"]
        );
        assert!(!control.is_reload_visible());
        assert_eq!(probe.changed.handler_count(), 1);
        control.on_detached_from_visual_tree();
        assert_eq!(probe.changed.handler_count(), 0);
        control.set_blocks_input(false); // Source recomputes from detached fields.
        assert!(!control.is_busy());
        assert!(control.reload_command().is_none());
    });
}

#[test]
fn dialog_shell_source_properties_keep_content_template_and_command_identity() {
    service_test(|| {
        use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
        use ferroui_reactiveui_controls::ReactiveCommandAdapter;
        let shell = DialogShellControl::new();
        assert!(shell.title().is_none());
        assert!(shell.content().is_none());
        assert!(shell.content_template().is_none());
        assert!(shell.close_command().is_none());
        assert!(shell.is_close_visible());
        shell.set_title(Some(String::new()));
        assert_eq!(shell.title().as_deref(), Some(""));
        let content = Border::new();
        shell.set_content(Some(Control::boxed(content.clone())));
        assert_eq!(
            Control::from_boxed(&shell.content().unwrap()),
            Some(content.upcast())
        );
        let template: Rc<dyn IDataTemplate> =
            FuncDataTemplate::new(|_| true, |_, _| Some(Border::new().upcast()), false);
        shell.set_content_template(Some(template.clone()));
        assert!(Rc::ptr_eq(&template, &shell.content_template().unwrap()));
        let command =
            ReactiveCommandAdapter::command_of(DockBusyService::new(None, None).reload_command());
        shell.set_close_command(Some(command.clone()));
        assert!(Rc::ptr_eq(&command, &shell.close_command().unwrap()));
        shell.set_is_close_visible(false);
        assert!(!shell.is_close_visible());
        shell.set_title(None);
        shell.set_content(None);
        shell.set_content_template(None);
        shell.set_close_command(None);
        assert!(shell.title().is_none());
        assert!(shell.content().is_none());
        assert!(shell.content_template().is_none());
        assert!(shell.close_command().is_none());
    });
}
#[test]
fn confirmation_shell_source_properties_preserve_nullable_text_and_native_commands() {
    service_test(|| {
        use ferroui_reactiveui_controls::ReactiveCommandAdapter;
        let shell = ConfirmationDialogControl::new();
        assert!(shell.title().is_none());
        assert!(shell.message().is_none());
        assert!(shell.confirm_text().is_none());
        assert!(shell.cancel_text().is_none());
        assert!(shell.confirm_command().is_none());
        assert!(shell.cancel_command().is_none());
        shell.set_title(Some("title".into()));
        shell.set_message(Some("message".into()));
        shell.set_confirm_text(Some("confirm".into()));
        shell.set_cancel_text(Some(String::new()));
        assert_eq!(shell.title().as_deref(), Some("title"));
        assert_eq!(shell.message().as_deref(), Some("message"));
        assert_eq!(shell.confirm_text().as_deref(), Some("confirm"));
        assert_eq!(shell.cancel_text().as_deref(), Some(""));
        let command =
            ReactiveCommandAdapter::command_of(DockBusyService::new(None, None).reload_command());
        shell.set_confirm_command(Some(command.clone()));
        shell.set_cancel_command(Some(command.clone()));
        assert!(Rc::ptr_eq(&command, &shell.confirm_command().unwrap()));
        assert!(Rc::ptr_eq(&command, &shell.cancel_command().unwrap()));
        shell.set_title(None);
        shell.set_message(None);
        shell.set_confirm_text(None);
        shell.set_cancel_text(None);
        shell.set_confirm_command(None);
        shell.set_cancel_command(None);
        assert!(shell.title().is_none());
        assert!(shell.message().is_none());
        assert!(shell.confirm_text().is_none());
        assert!(shell.cancel_text().is_none());
        assert!(shell.confirm_command().is_none());
        assert!(shell.cancel_command().is_none());
    });
}
