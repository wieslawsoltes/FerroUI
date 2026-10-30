//! Independent consumer of the Simple theme, using the original factory and models.
use super::{drain, Scene};
use ferrodock_ferro::{
    controls::{
        DockableControl, DocumentControl, DocumentTabStrip, HostWindowTitleBar, PinnedDockControl,
        ProportionalDockControl, RootDockControl, ToolTabStrip,
    },
    themes::DockDensityStyle,
};
use ferrodock_ferro_themes_simple::DockSimpleTheme;
use ferrodock_model::core::*;
use ferroui_base::{platform::AssetLoader, threading::CancellationToken, utilities::Uri, Ref};
use ferroui_controls::{AppBuilder, Application, ItemsControl, Panel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::io::Read;

fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_simple::SimpleTheme::new().as_style());
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "independent-simple-theme-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_simple::register_types();
                crate::compiled_markup::register();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

#[test]
fn downstream_simple_class_properties_and_compiled_resources_create_the_actual_title_bar() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/SimpleThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
        let style = panel.styles().get(0);
        let theme = style
            .as_object()
            .unwrap()
            .downcast_ref::<DockSimpleTheme>()
            .unwrap();
        assert_eq!(theme.density_style(), DockDensityStyle::Compact);
        assert!(theme.cache_document_tab_content());
        let title = panel
            .find_descendant_of_type::<HostWindowTitleBar>(false)
            .unwrap();
        let window = Window::new();
        window.set_content(Some(ferroui_controls::Control::boxed(panel.clone())));
        window.show();
        drain(&window);
        assert!(title.template().is_some());
        window.close();
    });
}

#[test]
fn simple_templates_render_engine_graph_cached_tabs_and_inline_pinned_tools() {
    run(|| {
        let scene = Scene::from_document(true, "SimpleDockingGraphConsumer.xaml");
        assert!(scene
            .control
            .find_descendant_of_type::<RootDockControl>(false)
            .is_some());
        assert!(scene
            .control
            .find_descendant_of_type::<ProportionalDockControl>(false)
            .is_some());
        let tabs = scene
            .control
            .find_descendant_of_type::<DocumentTabStrip>(false)
            .unwrap();
        assert_eq!(tabs.items().count(), 2);
        assert_eq!(
            scene
                .control
                .find_descendant_of_type::<ToolTabStrip>(false)
                .unwrap()
                .items()
                .count(),
            2
        );
        let document = scene
            .control
            .find_descendant_of_type::<DocumentControl>(false)
            .unwrap();
        let host = document
            .find_descendant_of_type_where::<ItemsControl>(false, |control| {
                control.name().as_deref() == Some("PART_CachedContentHost")
            })
            .unwrap();
        let first = host.container_from_index(0).unwrap();
        let second = host.container_from_index(1).unwrap();
        let first_view = first
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap();
        let second_view = second
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap();
        assert!(first_view.is_visible());
        assert!(!second_view.is_visible());
        scene
            .factory
            .set_active_dockable(scene.documents.visible_dockables().unwrap().get(1));
        drain(&scene.window);
        assert!(!first_view.is_visible());
        assert!(second_view.is_visible());
        assert!(host.container_from_index(0).unwrap().ptr_eq(&first));
        assert!(host.container_from_index(1).unwrap().ptr_eq(&second));
        assert!(scene
            .control
            .find_descendant_of_type::<PinnedDockControl>(false)
            .is_some());
        let pinned = scene.root.pinned_dock().unwrap();
        pinned.set_alignment(Alignment::Left);
        drain(&scene.window);
        assert!(scene
            .control
            .find_descendant_of_type::<PinnedDockControl>(false)
            .unwrap()
            .is_visible());
    });
}

#[test]
fn simple_assembly_embeds_all_original_linked_target_images_with_its_own_uris() {
    run(|| {
        for name in [
            "DockAnchorableTop",
            "DockAnchorableBottom",
            "DockAnchorableLeft",
            "DockAnchorableRight",
            "DockDocumentInside",
        ] {
            let uri = Uri::absolute(&format!(
                "ferres://FerroDock.Ferro.Themes.Simple/Assets/{name}.png"
            ))
            .unwrap();
            let mut asset = AssetLoader::open(&uri, None).unwrap();
            let mut signature = [0u8; 8];
            asset.read_exact(&mut signature).unwrap();
            assert_eq!(signature, [137, 80, 78, 71, 13, 10, 26, 10]);
        }
    });
}
