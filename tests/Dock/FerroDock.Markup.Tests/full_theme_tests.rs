//! Full original Fluent theme, density, document caching and host title-bar integration.
use ferrodock_ferro::{
    controls::HostWindowTitleBar, converters::IntLessThanConverter, themes::DockDensityStyle,
};
use ferrodock_ferro_themes_fluent::DockFluentTheme;
use ferroui_base::{
    controls::ResourceKey,
    data::{converters::IValueConverter, core::ValueType},
    threading::CancellationToken,
    utilities::CultureInfo,
    BoxedValue, Ref,
};
use ferroui_controls::{
    automation::{
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::IInvokeProvider,
    },
    AppBuilder, Application, Panel, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::{cell::Cell, rc::Rc};
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
    name: "complete-fluent-theme",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_fluent::register_types();
                crate::compiled_markup::register();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn number(theme: &DockFluentTheme, key: &str) -> f64 {
    *theme
        .try_get_resource(&ResourceKey::from(key), None)
        .unwrap()
        .unwrap()
        .downcast_ref::<f64>()
        .unwrap()
}
#[test]
fn complete_theme_preserves_source_resource_order_density_and_cache_override_lifetime() {
    run(|| {
        let theme = DockFluentTheme::new();
        assert_eq!(theme.density_style(), DockDensityStyle::Normal);
        assert!(!theme.cache_document_tab_content());
        assert!(theme
            .resources()
            .try_get_value(&ResourceKey::from("CompactStyles"))
            .is_none());
        assert_eq!(theme.resources().merged_dictionaries().count(), 50);
        let normal = number(&theme, "DockHeaderHeight");
        assert!(normal > 22.0);
        assert_eq!(theme.count(), 1);
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(number(&theme, "DockHeaderHeight"), 22.0);
        let provider = theme.as_resource_provider();
        assert_eq!(
            *provider
                .try_get_resource(&ResourceKey::from("DockHeaderHeight"), None)
                .unwrap()
                .unwrap()
                .downcast_ref::<f64>()
                .unwrap(),
            22.0
        );
        theme.set_density_style(DockDensityStyle::Normal);
        assert_eq!(number(&theme, "DockHeaderHeight"), normal);
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 2);
        let style = theme.get(1);
        let identity = style.reference_id();
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 2);
        assert_eq!(theme.get(1).reference_id(), identity);
        theme.set_cache_document_tab_content(false);
        assert_eq!(theme.count(), 1);
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 2);
        assert_ne!(theme.get(1).reference_id(), identity);
    });
}
#[test]
fn theme_property_changes_notify_the_resource_owner_only_when_values_change() {
    run(|| {
        let panel = Panel::new();
        let theme = DockFluentTheme::new();
        panel.styles().add(theme.as_style());
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let subscription = panel.resources_changed(move |_| count.set(count.get() + 1));
        let before = calls.get();
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(calls.get(), before + 1);
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(calls.get(), before + 1);
        let before = calls.get();
        theme.set_cache_document_tab_content(true);
        assert!(calls.get() > before);
        let before = calls.get();
        theme.set_cache_document_tab_content(true);
        assert_eq!(calls.get(), before);
        theme.set_cache_document_tab_content(false);
        assert!(calls.get() > before);
        subscription.dispose();
    });
}
#[test]
fn downstream_full_theme_constructs_title_bar_template_and_native_automation() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/FullThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
        let theme = panel.styles().get(0);
        let theme = theme
            .as_object()
            .unwrap()
            .downcast_ref::<DockFluentTheme>()
            .unwrap();
        assert_eq!(theme.density_style(), DockDensityStyle::Compact);
        assert!(theme.cache_document_tab_content());
        let title = panel
            .find_descendant_of_type::<HostWindowTitleBar>(false)
            .unwrap();
        let peer = ControlAutomationPeer::create_peer_for_element(&title);
        assert_eq!(peer.get_class_name(), "HostWindowTitleBar");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::TitleBar
        );
        assert_eq!(peer.get_name(), "Host window title bar");
        assert!(peer.get_help_text().contains("HasBackground=false"));
        let window = Window::new();
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        window.update_layout();
        assert!(peer.get_help_text().contains("HasBackground=true"));
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert!(title.is_focused());
        window.close();
    });
}
#[test]
fn integer_threshold_converter_preserves_exact_integer_and_failure_contract() {
    let converter = IntLessThanConverter::new();
    assert_eq!(converter.true_if_less_than(), 0);
    let culture = CultureInfo::invariant_culture();
    let convert = |value: Option<BoxedValue>| {
        *converter
            .convert(value.as_ref(), ValueType::of::<bool>(), None, &culture)
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>()
            .unwrap()
    };
    assert!(convert(Some(Rc::new(-1_i32))));
    assert!(!convert(Some(Rc::new(0_i32))));
    converter.set_true_if_less_than(2);
    assert!(convert(Some(Rc::new(1_i32))));
    assert!(!convert(Some(Rc::new(2_i32))));
    assert!(!convert(None));
    assert!(!convert(Some(Rc::new(1_i64))));
    assert!(!convert(Some(Rc::new("1".to_string()))));
    assert!(converter
        .convert_back(None, ValueType::of::<i32>(), None, &culture)
        .is_err());
}

#[test]
fn original_mdi_window_template_routes_minimize_maximize_and_restore_to_the_model() {
    run(|| {
        use ferrodock_ferro::controls::{MdiDocumentWindow, MdiLayoutPanel};
        use ferrodock_model::core::{IDockable, MdiWindowState};
        use ferrodock_model_reactiveui::controls::Document;
        use ferroui_base::{interactivity::RoutedEventArgs, threading::Dispatcher};
        use ferroui_controls::{Button, Control};
        let model = Document::new();
        model.set_title("Original MDI document".into());
        let mdi = MdiDocumentWindow::new();
        mdi.set_data_context(Some(
            ferrodock_ferro::model_projection::to_mdi_document_value(model.clone()),
        ));
        let panel = MdiLayoutPanel::new();
        panel.children().add(mdi.clone().upcast::<Control>());
        let window = Window::new();
        window.styles().add(DockFluentTheme::new().as_style());
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        for _ in 0..4 {
            Dispatcher::ui_thread().run_jobs(None);
            window.update_layout();
        }
        let click = |name: &str| {
            let button = mdi
                .find_descendant_of_type_where::<Button>(false, |button| {
                    button.name().as_deref() == Some(name)
                })
                .unwrap();
            let event = RoutedEventArgs::new();
            event.set_routed_event(Some(Button::click_event()));
            event.set_source(button.clone());
            button.raise_event(&event);
            window.update_layout();
        };
        click("PART_MinimizeButton");
        assert_eq!(model.mdi_state(), MdiWindowState::Minimized);
        click("PART_MaximizeRestoreButton");
        assert_eq!(model.mdi_state(), MdiWindowState::Normal);
        click("PART_MaximizeRestoreButton");
        assert_eq!(model.mdi_state(), MdiWindowState::Maximized);
        click("PART_MaximizeRestoreButton");
        assert_eq!(model.mdi_state(), MdiWindowState::Normal);
        window.close();
    });
}

#[test]
fn public_loader_retains_root_theme_and_all_original_preset_exports() {
    run(|| {
        use ferroui_base::utilities::Uri;
        use ferroui_markup_xaml::FerroXamlLoader;
        let theme = FerroXamlLoader::load(
            &Uri::absolute("ferres://FerroDock.Ferro.Themes.Fluent/DockFluentTheme.xaml").unwrap(),
            None,
        )
        .unwrap();
        assert!(theme.downcast_ref::<Ref<DockFluentTheme>>().is_some());
        for name in [
            "Default",
            "RiderDark",
            "RiderLight",
            "VsCodeDark",
            "VsCodeLight",
        ] {
            let value = FerroXamlLoader::load(
                &Uri::absolute(&format!(
                    "ferres://FerroDock.Ferro.Themes.Fluent/Presets/Ide/{name}.xaml"
                ))
                .unwrap(),
                None,
            )
            .unwrap();
            assert!(value
                .downcast_ref::<Ref<ferroui_base::controls::ResourceDictionary>>()
                .is_some());
        }
    });
}

#[test]
fn original_host_window_theme_binds_topmost_to_the_live_dock_window_model() {
    run(|| {
        use ferrodock_ferro::controls::HostWindow;
        use ferrodock_model::core::IDockable;
        use ferrodock_model_reactiveui::{controls::RootDock, core::DockWindow};
        use ferroui_base::threading::Dispatcher;
        let root = RootDock::new();
        root.set_id("FloatingRoot".into());
        let model = DockWindow::new();
        model.set_topmost(true);
        model.set_layout(Some(root.clone()));
        root.set_window(Some(model.clone()));
        let host = HostWindow::new();
        host.styles().add(DockFluentTheme::new().as_style());
        host.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
            root.clone(),
        )));
        host.show();
        for _ in 0..4 {
            Dispatcher::ui_thread().run_jobs(None);
            host.update_layout();
        }
        assert!(host.topmost());
        model.set_topmost(false);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!host.topmost());
        assert!(host
            .find_descendant_of_type::<ferrodock_ferro::controls::DockControl>(false)
            .is_some());
        host.close();
    });
}
