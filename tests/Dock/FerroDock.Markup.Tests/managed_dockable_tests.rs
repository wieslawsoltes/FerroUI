//! Compiled bindings against the original abstract managed dockable contract.
use ferrodock_ferro::{
    controls::{ManagedDockableBase, ManagedDockableBaseImpl},
    impl_managed_dockable, model_projection,
};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::Ref;
use ferroui_controls::{
    AppBuilder, Application, Button, Control, Panel, TextBlock, TextBox, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::Cell;
use std::rc::Rc;
// A test subclass exercises the source abstract base without introducing a
// production substitute for the managed dock/document classes still being ported.
struct ConsumerModel {
    base: ManagedDockableBase,
}
impl_managed_dockable!(ConsumerModel);
impl ManagedDockableBaseImpl for ConsumerModel {}
impl ConsumerModel {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
        })
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-managed-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn native_compiled_bindings_read_original_state_follow_notifications_rebind_and_write_through() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/ManagedDockableConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let title = panel.children().get(0).cast::<TextBlock>().unwrap();
                let id = panel.children().get(1).cast::<TextBlock>().unwrap();
                let close = panel.children().get(2).cast::<Button>().unwrap();
                let edit = panel.children().get(3).cast::<TextBox>().unwrap();
                let first = ConsumerModel::new();
                first.set_title(String::from("First"));
                first.set_id(String::from("first"));
                panel.set_data_context(Some(model_projection::to_managed_dockable_value(
                    first.clone(),
                )));
                let window = Window::new();
                window.set_width(600.0);
                window.set_height(400.0);
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                assert_eq!(title.text().as_deref(), Some("First"));
                assert_eq!(id.text().as_deref(), Some("first"));
                assert!(close.is_enabled());
                first.set_title(String::from("Changed"));
                first.set_can_close(false);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(title.text().as_deref(), Some("Changed"));
                assert!(!close.is_enabled());
                edit.set_current_value(TextBox::text_property(), Some(String::from("Edited")));
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(first.title(), "Edited");
                assert_eq!(title.text().as_deref(), Some("Edited"));
                let second = ConsumerModel::new();
                second.set_title(String::from("Second"));
                second.set_id(String::from("second"));
                panel.set_data_context(Some(model_projection::to_managed_dockable_value(
                    second.clone(),
                )));
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(title.text().as_deref(), Some("Second"));
                assert_eq!(id.text().as_deref(), Some("second"));
                assert!(close.is_enabled());
                first.set_title(String::from("Old"));
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(title.text().as_deref(), Some("Second"));
                edit.set_current_value(
                    TextBox::text_property(),
                    Some(String::from("Second edited")),
                );
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(second.title(), "Second edited");
                assert_eq!(first.title(), "Old");
                let changed = Rc::new(Cell::new(0));
                let observed = changed.clone();
                let token = second.property_changed().add(Rc::new(move |name| {
                    if name == "Context" {
                        observed.set(observed.get() + 1);
                    }
                }));
                let context = ferroui_controls::Border::new();
                second.set_context(Some(ferrodock_controls_recycling::to_recycling_value(
                    Rc::new(context.clone()),
                )));
                second.set_context(Some(ferrodock_controls_recycling::to_recycling_value(
                    Rc::new(context.upcast::<Control>()),
                )));
                assert_eq!(changed.get(), 1);
                second.property_changed().remove(token);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("original managed binding case completed");
}
