//! Original Browser overlay resources, cache, density and typed platform options.
use crate::BrowserTabTheme;
use ferrodock_ferro::themes::DockDensityStyle;
use ferrodock_ferro_themes_fluent::DockFluentTheme;
use ferroui_base::{
    controls::ResourceKey,
    data::core::ValueType,
    media::{Color, IBrush},
    metadata::{from_markup_value, MarkupTyped},
    threading::{CancellationToken, Dispatcher},
    Thickness,
};
use ferroui_controls::{AppBuilder, Application, Control, Panel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf;
use std::{cell::Cell, rc::Rc};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-browser-overlay",
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
                    .add(ferroui_themes_fluent::FluentTheme::new().as_style());
                app.resources().merged_dictionaries().clear();
                app.set_requested_theme_variant(None);
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn number(theme: &BrowserTabTheme, key: &str) -> f64 {
    *theme
        .try_get_resource(&ResourceKey::from(key), None)
        .unwrap()
        .unwrap()
        .downcast_ref::<f64>()
        .unwrap()
}
#[test]
fn source_overlay_keeps_embedded_fluent_and_full_selector_with_empty_compact_dictionary() {
    run(|| {
        let theme = BrowserTabTheme::new();
        assert_eq!(theme.resources().merged_dictionaries().count(), 2);
        assert_eq!(theme.count(), 22);
        assert!(theme
            .resources()
            .try_get_value(&ResourceKey::from("CompactStyles"))
            .is_none());
        let base = theme.get(0);
        let fluent = base
            .as_object()
            .unwrap()
            .downcast_ref::<DockFluentTheme>()
            .unwrap();
        assert_eq!(fluent.resources().merged_dictionaries().count(), 50);
        assert_eq!(
            theme
                .try_get_resource(
                    &ResourceKey::from("DockDocumentControlShowDockIndicatorOnly"),
                    None
                )
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&false)
        );
        for density in [DockDensityStyle::Normal, DockDensityStyle::Compact] {
            theme.set_density_style(density);
            assert_eq!(number(&theme, "DockDocumentTabItemMinHeight"), 28.0);
            assert_eq!(number(&theme, "DockTabItemMinHeight"), 24.0);
            assert_eq!(fluent.density_style(), DockDensityStyle::Normal);
        }
    });
}
#[test]
fn source_cache_override_and_density_notify_actual_owner_without_mutating_nested_fluent() {
    run(|| {
        let theme = BrowserTabTheme::with_service_provider(None);
        let panel = Panel::new();
        panel.styles().add(theme.as_style());
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        let subscription = panel.resources_changed(move |_| counter.set(counter.get() + 1));
        let before = calls.get();
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(calls.get(), before + 1);
        theme.set_density_style(DockDensityStyle::Compact);
        assert_eq!(calls.get(), before + 1);
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 23);
        let style = theme.get(22);
        let identity = style.reference_id();
        let before = calls.get();
        theme.set_cache_document_tab_content(true);
        assert_eq!(calls.get(), before);
        assert_eq!(theme.get(22).reference_id(), identity);
        theme.set_cache_document_tab_content(false);
        assert_eq!(theme.count(), 22);
        theme.set_cache_document_tab_content(true);
        assert_ne!(theme.get(22).reference_id(), identity);
        let base = theme.get(0);
        assert!(!base
            .as_object()
            .unwrap()
            .downcast_ref::<DockFluentTheme>()
            .unwrap()
            .cache_document_tab_content());
        subscription.dispose();
    });
}
#[test]
fn original_browser_colors_refresh_for_real_native_light_and_dark_resource_variants() {
    run(|| {
        let app = Application::current().unwrap();
        app.styles().add(BrowserTabTheme::new().as_style());
        let panel = Panel::new();
        let binding =
            ferroui_markup_xaml::markup_extensions::DynamicResourceExtension::with_resource_key(
                Some(Rc::new("DockSurfaceSidebarBrush".to_string())),
            );
        panel.bind_binding(Panel::background_property().as_property(), &*binding);
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        for (variant, workbench, editor) in [
            (
                ferroui_base::styling::ThemeVariant::light(),
                "#FFF3F6FB",
                "#FFFFFFFF",
            ),
            (
                ferroui_base::styling::ThemeVariant::dark(),
                "#FF1C2230",
                "#FF20283A",
            ),
        ] {
            app.set_requested_theme_variant(Some(variant.clone()));
            for _ in 0..3 {
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
            }
            assert_eq!(
                panel
                    .background()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                Color::parse(workbench).unwrap()
            );
            for (key, color) in [
                ("DockSurfaceSidebarBrush", workbench),
                ("DockSurfaceEditorBrush", editor),
            ] {
                let brush = from_markup_value::<Rc<dyn IBrush>>(
                    &panel
                        .try_find_resource(&ResourceKey::from(key), Some(&variant))
                        .unwrap(),
                )
                .unwrap();
                assert_eq!(
                    brush.as_solid_color_brush().unwrap().color(),
                    Color::parse(color).unwrap(),
                    "{key}"
                );
            }
        }
        window.close();
    });
}
#[test]
fn platform_metadata_keeps_actual_native_closed_types_and_compiled_platform_values() {
    run(|| {
        let double = crate::OnPlatformDoubleMetadata::MARKUP;
        let thickness = crate::OnPlatformThicknessMetadata::MARKUP;
        assert_eq!(
            (double.handles[0])(),
            ValueType::of::<Rc<OnPlatformExtensionOf<f64>>>()
        );
        assert_eq!(
            (thickness.handles[0])(),
            ValueType::of::<Rc<OnPlatformExtensionOf<Thickness>>>()
        );
        for metadata in [double, thickness] {
            assert_eq!(metadata.properties.len(), 7);
            for name in [
                "Default", "Windows", "macOS", "Linux", "Android", "iOS", "Browser",
            ] {
                assert!(metadata.find_property(name).unwrap().set.is_some());
            }
        }
        let theme = BrowserTabTheme::new();
        let mac = OnPlatformExtensionOf::<f64>::should_provide_option("OSX");
        assert_eq!(
            number(&theme, "DockDocumentTabStripLeadingSpacerWidth"),
            if mac { 70.0 } else { 12.0 }
        );
        let padding = theme
            .try_get_resource(&ResourceKey::from("DockHeaderContentPadding"), None)
            .unwrap()
            .unwrap();
        assert_eq!(
            padding.downcast_ref::<Thickness>(),
            Some(&if mac {
                Thickness::new(2.0, 1.0, 2.0, 0.0)
            } else {
                Thickness::uniform(2.0)
            })
        );
    });
}

#[repr(C)]
struct DerivedTheme {
    base: BrowserTabTheme,
}
ferroui_base::ferro_class!(DerivedTheme:BrowserTabTheme);
ferroui_base::ferro_class_info!(DerivedTheme {});
ferroui_base::ferro_impl_classes!(DerivedTheme:ferroui_base::FerroObjectImpl);
#[test]
fn source_theme_constructor_loads_resources_for_native_derived_instances() {
    run(|| {
        let theme = ferroui_base::instantiate(DerivedTheme {
            base: BrowserTabTheme::construct(),
        });
        assert_eq!(theme.resources().merged_dictionaries().count(), 2);
        theme.set_density_style(ferrodock_ferro::themes::DockDensityStyle::Compact);
        assert!(theme
            .try_get_resource(
                &ferroui_base::controls::ResourceKey::from("DockTabItemMinHeight"),
                None
            )
            .is_some());
        theme.set_cache_document_tab_content(true);
        assert_eq!(theme.count(), 23);
    });
}
