//! Separate native markup consumer using the original docking-target themes.
use ferrodock_ferro::controls::{DockAdornerWindow, DockTarget, GlobalDockTarget};
use ferrodock_model::core::{DockOperation, DragAction};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Point, Ref};
use ferroui_controls::{AppBuilder, Application, Control, Image, Panel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
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
    name: "ferrodock-target-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn exported_targets_use_original_fluent_templates_images_bindings_and_window_theme() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferrodock_controls_proportional_stack_panel::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/DockTargetConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let local = panel.children().get(0).cast::<DockTarget>().unwrap();
                let global = panel.children().get(1).cast::<GlobalDockTarget>().unwrap();
                let window = Window::new();
                window.set_width(400.0);
                window.set_height(300.0);
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                for operation in [
                    DockOperation::Top,
                    DockOperation::Bottom,
                    DockOperation::Left,
                    DockOperation::Right,
                    DockOperation::Fill,
                ] {
                    let selector = local.get_selector(operation).unwrap();
                    let image = selector.cast::<Image>().unwrap();
                    assert!(image.source().is_some());
                    assert_eq!(image.width(), 40.0);
                    assert_eq!(image.height(), 40.0);
                    let point = selector
                        .translate_point(Point::new(20.0, 20.0), &local)
                        .unwrap();
                    assert_eq!(
                        local.get_dock_operation(
                            point,
                            &local,
                            &local,
                            DragAction::MOVE,
                            &|_, _, _, _| true,
                            None
                        ),
                        operation
                    );
                }
                for operation in [
                    DockOperation::Top,
                    DockOperation::Bottom,
                    DockOperation::Left,
                    DockOperation::Right,
                ] {
                    let selector = global.get_selector(operation).unwrap();
                    let point = selector
                        .translate_point(Point::new(20.0, 20.0), &global)
                        .unwrap();
                    assert_eq!(
                        global.get_dock_operation(
                            point,
                            &global,
                            &global,
                            DragAction::MOVE,
                            &|_, _, _, _| true,
                            None
                        ),
                        operation
                    );
                }
                let right = local.get_selector(DockOperation::Right).unwrap();
                local.set_show_horizontal_targets(false);
                assert!(!right.is_visible());
                local.set_show_horizontal_targets(true);
                assert!(right.is_visible());
                assert!(global.get_selector(DockOperation::Fill).is_none());
                global.set_show_horizontal_targets(false);
                assert!(!global
                    .get_selector(DockOperation::Right)
                    .unwrap()
                    .is_visible());
                assert!(global
                    .get_selector(DockOperation::Top)
                    .unwrap()
                    .is_visible());
                local.set_show_indicators_only(true);
                let selector_panel = local
                    .find_descendant_of_type_where::<Panel>(false, |p| {
                        p.name().as_deref() == Some("PART_SelectorPanel")
                    })
                    .unwrap();
                assert!(!selector_panel.is_visible());
                local.set_show_indicators_only(false);
                assert!(selector_panel.is_visible());
                let host = DockAdornerWindow::new();
                let theme = panel.find_resource(&ferroui_base::controls::ResourceKey::from(
                    <DockAdornerWindow as ferroui_base::StaticType>::TYPE,
                ));
                let theme = ferroui_base::metadata::from_markup_value::<
                    Ref<ferroui_base::styling::ControlTheme>,
                >(&theme)
                .unwrap();
                host.set_theme(Some(theme));
                host.apply_styling();
                assert!(!host.show_in_taskbar());
                assert!(!host.can_resize());
                assert!(!host.show_activated());
                assert!(!host.topmost());
                assert!(!host.is_hit_test_visible());
                assert_eq!(
                    host.window_decorations(),
                    ferroui_controls::WindowDecorations::None
                );
                host.close();
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
