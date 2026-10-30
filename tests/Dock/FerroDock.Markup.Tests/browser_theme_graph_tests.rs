//! Original Browser tab selectors, delayed layout and floating-host integration.
use super::{drain, Scene};
use ferrodock_ferro::controls::{
    DelayedUniformTabPanel, DockableControl, DocumentControl, DocumentTabStrip,
    DocumentTabStripItem, HostWindow,
};
use ferrodock_model::{collections::ListHandle, core::*};
use ferrodock_model_reactiveui::controls::{RootDock, Tool, ToolDock};
use ferroui_base::{
    input::{MouseButton, RawInputModifiers},
    threading::CancellationToken,
    Point, Thickness,
};
use ferroui_controls::{AppBuilder, Application, Border, ItemsControl, WindowDecorations};
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
                .add(ferroui_themes_fluent::FluentTheme::new().as_style());
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "independent-browser-overlay-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_browser::register_types();
                crate::compiled_markup::register();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
#[test]
fn browser_uses_actual_delayed_panel_and_native_pointer_selection_preserves_cached_views() {
    run(|| {
        let scene = Scene::from_document(true, "BrowserDockingGraphConsumer.xaml");
        let tabs = scene
            .control
            .find_descendant_of_type::<DocumentTabStrip>(false)
            .unwrap();
        let panel = tabs
            .find_descendant_of_type::<DelayedUniformTabPanel>(false)
            .unwrap();
        assert_eq!(panel.max_tab_width(), 220.0);
        assert_eq!(panel.min_tab_width(), 28.0);
        assert_eq!(panel.item_spacing(), -6.0);
        assert_eq!(
            panel.expansion_delay(),
            ferroui_base::animation::TimeSpan::from_seconds(1.0)
        );
        assert_eq!(
            panel.resize_animation_duration(),
            ferroui_base::animation::TimeSpan::from_milliseconds(180.0)
        );
        let document = scene
            .control
            .find_descendant_of_type::<DocumentControl>(false)
            .unwrap();
        let target = document
            .find_descendant_of_type_where::<ferroui_controls::DockPanel>(false, |control| {
                control.name().as_deref() == Some("PART_DockPanel")
            })
            .unwrap();
        assert!(!ferrodock_settings::DockProperties::get_show_dock_indicator_only(&target));
        let host = document
            .find_descendant_of_type_where::<ItemsControl>(false, |control| {
                control.name().as_deref() == Some("PART_CachedContentHost")
            })
            .unwrap();
        let first = host.container_from_index(0).unwrap();
        let second = host.container_from_index(1).unwrap();
        let item = tabs
            .container_from_index(1)
            .unwrap()
            .cast::<DocumentTabStripItem>()
            .unwrap();
        let point = item
            .translate_point(
                Point::new(item.bounds().width / 2.0, item.bounds().height / 2.0),
                &scene.window,
            )
            .unwrap();
        scene.window.mouse_move(point, RawInputModifiers::NONE);
        scene
            .window
            .mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        scene
            .window
            .mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        drain(&scene.window);
        assert_eq!(scene.documents.active_dockable().unwrap().id(), "Doc2");
        assert!(host.container_from_index(0).unwrap().ptr_eq(&first));
        assert!(host.container_from_index(1).unwrap().ptr_eq(&second));
        assert!(!first
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap()
            .is_visible());
        assert!(second
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap()
            .is_visible());
    });
}
#[test]
fn browser_original_selector_matrix_keeps_gutter_body_and_close_presenter_states() {
    run(|| {
        let scene = Scene::from_document(false, "BrowserDockingGraphConsumer.xaml");
        let tabs = scene
            .control
            .find_descendant_of_type::<DocumentTabStrip>(false)
            .unwrap();
        let item = tabs
            .container_from_index(0)
            .unwrap()
            .cast::<DocumentTabStripItem>()
            .unwrap();
        assert!(!item.clip_to_bounds());
        let border = |name: &str| {
            item.find_descendant_of_type_where::<Border>(false, |control| {
                control.name().as_deref() == Some(name)
            })
            .unwrap()
        };
        let gutter = border("PART_TabGutter");
        let body = border("PART_TabBody");
        let separator = border("PART_TabSeparatorRight");
        let close = item
            .find_descendant_of_type_where::<ferroui_controls::presenters::ContentPresenter>(
                false,
                |control| control.name().as_deref() == Some("PART_ClosePresenter"),
            )
            .unwrap();
        for (selected, hover, dragging) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (true, true, false),
            (true, false, true),
        ] {
            item.pseudo_classes().set(":selected", selected);
            item.pseudo_classes().set(":pointerover", hover);
            item.pseudo_classes().set(":dragging", dragging);
            assert_eq!(separator.is_visible(), !(selected || hover || dragging));
            assert_eq!(close.is_visible(), selected || hover);
            if hover {
                assert_eq!(item.z_index(), 2);
            } else if selected {
                assert_eq!(item.z_index(), 3);
            }
            assert_eq!(
                gutter.margin(),
                if hover || dragging {
                    Thickness::new(-4.0, 0.0, -2.0, 0.0)
                } else if selected {
                    Thickness::new(-3.0, 0.0, -2.0, 0.0)
                } else {
                    Thickness::default()
                }
            );
            assert_eq!(
                body.margin(),
                if hover || dragging {
                    Thickness::new(7.0, 0.0, 5.0, 0.0)
                } else if selected {
                    Thickness::new(6.0, 0.0, 5.0, 0.0)
                } else {
                    Thickness::new(3.0, 0.0, 3.0, 0.0)
                }
            );
        }
        item.pseudo_classes().set(":dragging", false);
        item.pseudo_classes().set(":pointerover", false);
        item.pseudo_classes().set(":selected", true);
        item.pseudo_classes().set(":ultracompact", true);
        let icon = item
            .find_descendant_of_type_where::<ferroui_controls::presenters::ContentPresenter>(
                false,
                |control| control.name().as_deref() == Some("PART_IconPresenter"),
            )
            .unwrap();
        assert!(!icon.is_visible());
        let compact = item
            .find_descendant_of_type_where::<ferroui_controls::presenters::ContentPresenter>(
                false,
                |control| control.name().as_deref() == Some("PART_CompactClosePresenter"),
            )
            .unwrap();
        assert!(compact.is_visible());
        let scroll = tabs
            .find_descendant_of_type_where::<ferroui_controls::ScrollViewer>(false, |control| {
                control.name().as_deref() == Some("PART_ScrollViewer")
            })
            .unwrap();
        assert_eq!(
            scroll.horizontal_scroll_bar_visibility(),
            ferroui_controls::primitives::ScrollBarVisibility::Disabled
        );
    });
}
#[test]
fn original_browser_multi_tab_tool_host_retains_full_decorations_and_regular_presenter() {
    run(|| {
        let app = Application::current().unwrap();
        app.styles()
            .add(ferrodock_ferro_themes_browser::BrowserTabTheme::new().as_style());
        let tools = ToolDock::new();
        let items = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        for name in ["Tool1", "Tool2"] {
            let tool = Tool::new();
            tool.set_title(name.into());
            items.add(Some(tool));
        }
        tools.set_visible_dockables(Some(items.clone()));
        tools.set_active_dockable(items.get(0));
        tools.set_opened_dockables_count(2);
        let root = RootDock::new();
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            tools.clone() as Rc<dyn IDockable>
        )])));
        root.set_active_dockable(Some(tools.clone()));
        root.set_default_dockable(Some(tools));
        root.set_opened_dockables_count(2);
        let host = HostWindow::new();
        host.set_width(400.0);
        host.set_height(300.0);
        host.set_layout(root);
        host.show();
        drain(&host);
        assert!(host.is_tool_window());
        assert!(!host.tool_chrome_controls_whole_window());
        assert!(host.extend_client_area_to_decorations_hint());
        assert_eq!(host.window_decorations(), WindowDecorations::Full);
        let presenter = host
            .find_descendant_of_type_where::<ferroui_controls::presenters::ContentPresenter>(
                false,
                |control| control.name().as_deref() == Some("PART_ContentPresenter"),
            )
            .unwrap();
        assert_eq!(presenter.get_type().name(), "ContentPresenter");
        assert_eq!(host.padding(), Thickness::uniform(6.0));
        host.close();
    });
}
