//! Separate consumer exercising the exported root class with a test-only template.
use ferrodock_ferro::{controls::RootDockControl, model_projection};
use ferrodock_model::{collections::ListHandle, controls::*, core::*};
use ferrodock_model_reactiveui::{
    controls::{RootDock, Tool, ToolDock},
    Factory,
};
use ferroui_base::{
    input::{MouseButton, RawInputModifiers},
    threading::{CancellationToken, Dispatcher},
    Point, Ref,
};
use ferroui_controls::{
    automation::peers::{AutomationControlType, ControlAutomationPeer},
    AppBuilder, Application, Control, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::rc::Rc;

fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-external-root-control",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
struct Scene(Ref<Window>);
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
#[test]
fn external_compiled_root_template_uses_original_model_engine_and_native_peer() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/RootControlConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let control = value.downcast_ref::<Ref<RootDockControl>>().unwrap();
                let root = RootDock::new();
                root.set_id("Root".into());
                root.set_title("Original root".into());
                let factory = Factory::new();
                root.set_factory(Some(factory.clone()));
                let pinned = ToolDock::new();
                let tool = Tool::new();
                tool.set_owner(Some(pinned.clone()));
                pinned.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(
                    tool as Rc<dyn IDockable>,
                )])));
                root.set_pinned_dock(Some(pinned));
                control.set_data_context(Some(model_projection::to_root_value(root.clone())));
                let scene = Scene(Window::new());
                scene.0.set_width(640.0);
                scene.0.set_height(360.0);
                scene.0.set_content(Some(Control::boxed(control.clone())));
                scene.0.show();
                Dispatcher::ui_thread().run_jobs(None);
                scene.0.update_layout();
                let peer = ControlAutomationPeer::create_peer_for_element(control);
                assert_eq!(peer.get_class_name(), "RootDockControl");
                assert_eq!(
                    peer.get_automation_control_type(),
                    AutomationControlType::Pane
                );
                assert_eq!(peer.get_name(), "Original root");
                assert_eq!(peer.get_automation_id().as_deref(), Some("Root"));
                assert!(peer.get_help_text().contains("PinnedVisible=1"));
                scene.0.mouse_down(
                    Point::new(20.0, 20.0),
                    MouseButton::Left,
                    RawInputModifiers::empty(),
                );
                scene.0.mouse_up(
                    Point::new(20.0, 20.0),
                    MouseButton::Left,
                    RawInputModifiers::empty(),
                );
                assert!(root.pinned_dock().is_none());
                assert!(peer.get_help_text().contains("PinnedVisible=0"));
                let context = control.data_context().unwrap();
                let current = context.downcast_ref::<Rc<dyn IRootDock>>().unwrap();
                assert!(Rc::ptr_eq(current, &(root.clone() as Rc<dyn IRootDock>)));
                let dockable = model_projection::to_dockable_value(root.clone());
                let original = dockable.downcast_ref::<Rc<dyn IDockable>>().unwrap();
                assert!(Rc::ptr_eq(original, &(root.clone() as Rc<dyn IDockable>)));
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
