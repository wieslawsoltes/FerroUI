use ferrodock_ferro::controls::overlays::{
    BusyOverlayControl, ConfirmationOverlayControl, DialogOverlayControl,
};
use ferrodock_model::services::*;
use ferrodock_model_reactiveui_services::overlays::services::*;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Ref, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{
    AppBuilder, Application, Button, Control, Grid, ItemsControl, ProgressBar, TextBlock, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::threading::{
    current_synchronization_context, set_synchronization_context, tasks::Task,
    SynchronizationContext,
};
use std::rc::Rc;
pub(super) struct Environment(
    Option<std::sync::Arc<dyn SynchronizationContext>>,
    Rc<dyn IScheduler>,
);
impl Environment {
    pub(super) fn new() -> Self {
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
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            let application = Application::current().unwrap();
            application
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style());
        })
}
pub(super) const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-service-overlay-markup",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
pub(super) fn descendants(root: &Visual) -> Vec<Ref<Control>> {
    let mut result = Vec::new();
    for child in root.visual_children().snapshot().iter() {
        if let Some(control) = child.cast::<Control>() {
            result.push(control);
        }
        result.extend(descendants(child));
    }
    result
}
#[test]
fn original_overlay_templates_compile_bind_and_follow_live_services_in_a_downstream_assembly() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let _environment = Environment::new();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/ServiceOverlayConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let grid = value.downcast_ref::<Ref<Grid>>().unwrap();
                let busy = grid.children().get(0).cast::<BusyOverlayControl>().unwrap();
                let dialog = grid
                    .children()
                    .get(1)
                    .cast::<DialogOverlayControl>()
                    .unwrap();
                let confirmation = grid
                    .children()
                    .get(2)
                    .cast::<ConfirmationOverlayControl>()
                    .unwrap();
                let busy_service = DockBusyService::new(None, None);
                let dialog_service = DockDialogService::new(None, None);
                let confirmation_service = DockConfirmationService::new(None, None);
                busy.set_busy_service(Some(busy_service.clone()));
                dialog.set_dialog_service(Some(dialog_service.clone()));
                confirmation.set_confirmation_service(Some(confirmation_service.clone()));
                let window = Window::new();
                window.set_content(Some(Control::boxed(grid.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let busy_children = descendants(&busy);
                let reload = busy_children
                    .iter()
                    .find_map(|c| c.cast::<Button>())
                    .unwrap();
                let progress = busy_children
                    .iter()
                    .find_map(|c| c.cast::<ProgressBar>())
                    .unwrap();
                let template_root = busy.visual_children().get(0).cast::<Grid>().unwrap();
                let overlay_children = descendants(&template_root.children().get(2));
                let text = overlay_children
                    .iter()
                    .find_map(|c| c.cast::<TextBlock>())
                    .unwrap();
                let presenter = busy_children
                    .iter()
                    .find_map(|c| c.cast::<ContentPresenter>())
                    .unwrap();
                assert!(presenter.child().is_some());
                assert_eq!(text.text().as_deref(), Some("Loading..."));
                assert!(reload.command().is_some());
                let calls = Rc::new(std::cell::Cell::new(0));
                let called = calls.clone();
                busy_service.set_reload_handler(Some(Rc::new(move || {
                    called.set(called.get() + 1);
                    Task::from_result(())
                })));
                assert!(reload.is_enabled());
                reload.command().unwrap().execute(None);
                assert_eq!(calls.get(), 1);
                let scope = busy_service.begin("compiled busy".into());
                assert!(!reload.is_enabled());
                assert!(progress.is_visible());
                assert_eq!(text.text().as_deref(), Some("compiled busy"));
                assert!(!presenter.is_enabled());
                assert!(!presenter.is_hit_test_visible());
                scope.dispose();
                assert!(!progress.is_visible());
                assert!(presenter.is_enabled());
                let items = descendants(&dialog)
                    .iter()
                    .find_map(|c| c.cast::<ItemsControl>())
                    .unwrap();
                let confirmations = descendants(&confirmation)
                    .iter()
                    .find_map(|c| c.cast::<ItemsControl>())
                    .unwrap();
                let source = items.items_source().unwrap();
                let confirmation_source = confirmations.items_source().unwrap();
                assert_eq!(source.identity(), dialog_service.dialogs().identity());
                assert_eq!(
                    confirmation_source.identity(),
                    confirmation_service.confirmations().identity()
                );
                let _dialog_task =
                    dialog_service.show_async_boxed(DialogContent::new(Rc::new(5)), None);
                let _confirmation_task =
                    confirmation_service.confirm_async_default("title".into(), "message".into());
                assert_eq!(items.item_count(), 1);
                assert_eq!(confirmations.item_count(), 1);
                assert!(items.is_visible());
                assert!(confirmations.is_visible());
                let item = source.get_at(0).unwrap();
                assert!(Rc::ptr_eq(
                    item.downcast_ref::<Rc<DialogRequest>>().unwrap(),
                    &dialog_service.dialogs().get(0)
                ));
                dialog_service.cancel_all();
                confirmation_service.cancel_all();
                assert_eq!(items.item_count(), 0);
                assert_eq!(confirmations.item_count(), 0);
                assert!(!items.is_visible());
                assert!(!confirmations.is_visible());
                dialog.set_dialog_service(None);
                confirmation.set_confirmation_service(None);
                busy.set_busy_service(None);
                assert!(items.items_source().is_none());
                assert!(confirmations.items_source().is_none());
                assert!(reload.command().is_none());
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
