use ferrodock_ferro::behaviors::VisualTreeLifecycleBehavior as Behavior;
use ferrodock_model::services::IVisualTreeLifecycle;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Application, ContentControl, Control, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::Cell;
use std::rc::Rc;
struct Model {
    attached: Cell<usize>,
    detached: Cell<usize>,
}
impl IVisualTreeLifecycle for Model {
    fn on_attached_to_visual_tree(&self) {
        self.attached.set(self.attached.get() + 1);
    }
    fn on_detached_from_visual_tree(&self) {
        self.detached.set(self.detached.get() + 1);
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-lifecycle-markup",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn external_compiled_attached_behavior_invokes_the_original_domain_contract_across_reattachment() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/LifecycleConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let control = value.downcast_ref::<Ref<ContentControl>>().unwrap();
                assert!(Behavior::get_is_enabled(control));
                let model = Rc::new(Model {
                    attached: Cell::new(0),
                    detached: Cell::new(0),
                });
                control
                    .set_data_context(Some(Rc::new(model.clone() as Rc<dyn IVisualTreeLifecycle>)));
                assert_eq!(model.attached.get(), 0);
                let window = Window::new();
                window.set_content(Some(Control::boxed(control.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                assert_eq!(model.attached.get(), 1);
                window.set_content(None);
                window.update_layout();
                assert_eq!(model.detached.get(), 1);
                window.set_content(Some(Control::boxed(control.clone())));
                window.update_layout();
                assert_eq!(model.attached.get(), 2);
                Behavior::set_is_enabled(control, false);
                assert_eq!(model.detached.get(), 2);
                window.close();
                assert_eq!(model.detached.get(), 2);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
