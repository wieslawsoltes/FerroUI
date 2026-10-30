//! Original Simple theme density, caching, preset and runtime resource scenarios.
use crate::{DockSimpleTheme, DockSimpleThemeManager};
use ferrodock_ferro::{
    controls::ToolChromeControl,
    themes::{DockDensityStyle, IDockThemeManager},
};
use ferroui_base::{
    controls::{ResourceDictionary, ResourceKey},
    media::{Color, VisualBrush},
    metadata::{from_markup_value, IServiceProvider},
    styling::ThemeVariant,
    threading::{CancellationToken, Dispatcher},
    utilities::Uri,
};
use ferroui_controls::{AppBuilder, Application, Control, Grid, Panel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_markup_xaml::styling::ResourceInclude;
use std::{cell::Cell, rc::Rc};

fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-simple-theme",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                let app = Application::current().unwrap();
                app.styles().clear();
                app.styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                app.resources().merged_dictionaries().clear();
                app.set_requested_theme_variant(None);
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn number(theme: &DockSimpleTheme, key: &str) -> f64 {
    *theme
        .try_get_resource(&ResourceKey::from(key), None)
        .unwrap()
        .unwrap()
        .downcast_ref::<f64>()
        .unwrap()
}
fn include(name: &str) -> Rc<ResourceInclude> {
    let uri = Uri::absolute(&format!(
        "ferres://FerroDock.Ferro.Themes.Simple/Presets/Ide/{name}.xaml"
    ))
    .unwrap();
    let value = ResourceInclude::new(Some(uri.clone()));
    value.set_source(Some(uri));
    value
}
fn preset_sources(dictionary: &ResourceDictionary) -> Vec<String> {
    dictionary
        .merged_dictionaries()
        .snapshot()
        .iter()
        .filter_map(|provider| {
            provider.as_any().and_then(|provider| provider.downcast_ref::<ResourceInclude>())
                .and_then(ResourceInclude::source)
                .map(|uri| uri.to_string())
        })
        .filter(|uri| uri.contains("/Presets/Ide/"))
        .collect()
}
fn drain(window: &Window) {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
    }
}

#[test]
fn original_simple_root_keeps_42_merged_resources_and_all_density_values() {
    run(|| {
        let theme = DockSimpleTheme::new();
        assert_eq!(theme.resources().merged_dictionaries().count(), 42);
        assert_eq!(theme.count(), 1);
        assert_eq!(theme.density_style(), DockDensityStyle::Normal);
        assert!(!theme.cache_document_tab_content());
        assert!(theme
            .resources()
            .try_get_value(&ResourceKey::from("CompactStyles"))
            .is_none());
        for (density, expected) in [
            (
                DockDensityStyle::Normal,
                [24.0, 0.0, 14.0, 12.0, 5.0, 2.0, 10.0, 14.0, 16.0],
            ),
            (
                DockDensityStyle::Compact,
                [22.0, 0.0, 12.0, 10.0, 4.0, 1.0, 9.0, 13.0, 14.0],
            ),
        ] {
            theme.set_density_style(density);
            for (key, expected) in [
                "DockTabItemMinHeight",
                "DockToolTabItemMinHeight",
                "DockCloseButtonSize",
                "DockMdiTitleIconSize",
                "DockChromeGripHeight",
                "DockTabContentSpacing",
                "DockDragPreviewStatusIconSize",
                "DockOverlayMessageFontSize",
                "DockDialogTitleFontSize",
            ]
            .into_iter()
            .zip(expected)
            {
                assert_eq!(number(&theme, key), expected, "{density:?}: {key}");
                let value = theme
                    .as_resource_provider()
                    .try_get_resource(&ResourceKey::from(key), None)
                    .unwrap()
                    .unwrap();
                assert_eq!(value.downcast_ref::<f64>(), Some(&expected));
            }
        }
        theme.set_density_style(DockDensityStyle::Normal);
        assert_eq!(number(&theme, "DockTabItemMinHeight"), 24.0);
    });
}

#[test]
fn simple_cache_override_retains_source_style_identity_and_owner_notifications() {
    run(|| {
        let panel = Panel::new();
        let theme = DockSimpleTheme::new();
        panel.styles().add(theme.as_style());
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        let subscription = panel.resources_changed(move |_| counter.set(counter.get() + 1));
        let before = calls.get();
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(calls.get(), before + 1);
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(calls.get(), before + 1);
        let before = calls.get();
        theme.set_cache_document_tab_content(true);
        assert!(calls.get() > before);
        assert_eq!(theme.count(), 2);
        let style = theme.get(1);
        let identity = style.reference_id();
        let before = calls.get();
        theme.set_cache_document_tab_content(true);
        assert_eq!(calls.get(), before);
        assert_eq!(theme.get(1).reference_id(), identity);
        theme.set_cache_document_tab_content(false);
        assert_eq!(theme.count(), 1);
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 2);
        assert_ne!(theme.get(1).reference_id(), identity);
        subscription.dispose();
    });
}

#[test]
fn simple_manager_preserves_shared_names_guards_and_original_default_owner_includes() {
    run(|| {
        let app = Application::current().unwrap();
        let theme = DockSimpleTheme::new();
        app.styles().add(theme.as_style());
        let manager = DockSimpleThemeManager::new();
        let other = DockSimpleThemeManager::new();
        let names = manager.preset_names();
        assert_eq!(
            (0..names.count()).map(|i| names.get(i)).collect::<Vec<_>>(),
            [
                "Default",
                "VS Code Light",
                "VS Code Dark",
                "Rider Light",
                "Rider Dark"
            ]
        );
        assert_eq!(names.identity(), other.preset_names().identity());
        assert_eq!(manager.current_preset_index(), 0);
        for index in [-1, 5, i32::MAX] {
            manager.switch_preset(index);
        }
        assert!(preset_sources(&app.resources()).is_empty());
        app.resources()
            .merged_dictionaries()
            .add(include("Default"));
        manager.switch_preset(2);
        for dictionary in [app.resources(), theme.resources()] {
            assert_eq!(
                preset_sources(&dictionary),
                [include("VsCodeDark").source().unwrap().to_string()]
            );
        }
        let provider = theme.resources().merged_dictionaries().get(42);
        manager.switch_preset(2);
        assert!(Rc::ptr_eq(
            &provider,
            &theme.resources().merged_dictionaries().get(42)
        ));
        manager.switch(0);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::light()));
        manager.switch(1);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::dark()));
        manager.switch(2);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::dark()));
    });
}

#[test]
fn all_five_simple_presets_load_and_runtime_switch_updates_real_surface_brushes() {
    run(|| {
        let app = Application::current().unwrap();
        let theme = DockSimpleTheme::new();
        app.styles().add(theme.as_style());
        let manager = DockSimpleThemeManager::new();
        let panel = Panel::new();
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        drain(&window);
        for (variant, default) in [(0, "#FFFFFFFF"), (1, "#FF282828")] {
            manager.switch(variant);
            for index in [0, 1, 2, 3, 4, 0] {
                manager.switch_preset(index);
                drain(&window);
                assert_eq!(manager.current_preset_index(), index);
                let expected = match index {
                    0 => [default, default],
                    1 => ["#FFF3F3F3", "#FFFFFFFF"],
                    2 => ["#FF252526", "#FF1E1E1E"],
                    3 => ["#FFF6F6F6", "#FFFFFFFF"],
                    4 => ["#FF2A2C2F", "#FF1F2124"],
                    _ => unreachable!(),
                };
                for (key, color) in ["DockSurfaceSidebarBrush", "DockSurfaceEditorBrush"]
                    .into_iter()
                    .zip(expected)
                {
                    let brush = from_markup_value::<Rc<dyn ferroui_base::media::IBrush>>(
                        &panel.find_resource(&ResourceKey::from(key)),
                    )
                    .unwrap();
                    assert_eq!(
                        brush.as_solid_color_brush().unwrap().color(),
                        Color::parse(color).unwrap(),
                        "variant={variant} preset={index} key={key}"
                    );
                }
            }
        }
        window.close();
    });
}

#[test]
fn simple_runtime_dark_preset_clears_the_actual_default_grip_visual_brush() {
    run(|| {
        let app = Application::current().unwrap();
        app.styles().add(DockSimpleTheme::new().as_style());
        let nested = ResourceDictionary::new();
        nested.merged_dictionaries().add(include("Default"));
        let outer = ResourceDictionary::new();
        outer.merged_dictionaries().add(nested.into());
        app.resources().merged_dictionaries().add(outer.into());
        let chrome = ToolChromeControl::new();
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(300.0);
        window.set_content(Some(Control::boxed(chrome.clone())));
        window.show();
        drain(&window);
        let grid = || {
            chrome
                .find_descendant_of_type_where::<Grid>(false, |grid| {
                    grid.name().as_deref() == Some("PART_Grid")
                })
                .unwrap()
        };
        let before = grid().background().unwrap();
        assert!(before
            .as_object()
            .unwrap()
            .downcast_ref::<VisualBrush>()
            .is_some());
        let manager = DockSimpleThemeManager::new();
        for index in [2, 0, 4] {
            manager.switch_preset(index);
            drain(&window);
            let brush = grid().background().unwrap();
            assert_eq!(
                brush
                    .as_object()
                    .and_then(|object| object.downcast_ref::<VisualBrush>())
                    .is_some(),
                index == 0
            );
        }
        window.close();
    });
}

#[test]
fn simple_optional_service_provider_constructor_populates_the_original_class() {
    run(|| {
        let provider: Option<Rc<dyn IServiceProvider>> = None;
        let theme = DockSimpleTheme::with_service_provider(provider);
        assert_eq!(
            theme.get_type().full_name(),
            "FerroDock.Ferro.Themes.Simple.DockSimpleTheme"
        );
        assert_eq!(theme.resources().merged_dictionaries().count(), 42);
        let style = theme.as_style();
        assert_eq!(
            style.reference_id(),
            theme.as_resource_provider().reference_id()
        );
        assert!(style
            .as_object()
            .unwrap()
            .downcast_ref::<DockSimpleTheme>()
            .is_some());
    });
}

#[repr(C)]
struct DerivedTheme {
    base: DockSimpleTheme,
}
ferroui_base::ferro_class!(DerivedTheme:DockSimpleTheme);
ferroui_base::ferro_class_info!(DerivedTheme {});
ferroui_base::ferro_impl_classes!(DerivedTheme:ferroui_base::FerroObjectImpl);
#[test]
fn source_theme_constructor_loads_resources_for_native_derived_instances() {
    run(|| {
        let theme = ferroui_base::instantiate(DerivedTheme {
            base: DockSimpleTheme::construct(),
        });
        assert_eq!(theme.resources().merged_dictionaries().count(), 42);
        theme.set_density_style(ferrodock_ferro::themes::DockDensityStyle::Compact);
        assert!(theme
            .try_get_resource(
                &ferroui_base::controls::ResourceKey::from("DockTabItemMinHeight"),
                None
            )
            .is_some());
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 2);
    });
}
