//! Separate compiled consumer for preview properties and the original window theme.
use ferrodock_ferro::controls::{DragPreviewControl, DragPreviewWindow};
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{AppBuilder, Application, ContentControl, Panel, TextBlock, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
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
    name: "ferrodock-preview-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn compiled_preview_properties_update_native_template_and_original_window_theme_keeps_every_setter()
{
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/DragPreviewConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let preview = panel
                    .children()
                    .get(0)
                    .cast::<DragPreviewControl>()
                    .unwrap();
                let window = Window::new();
                window.set_content(Some(Rc::new(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let status = preview
                    .find_descendant_of_type_where::<TextBlock>(false, |c| {
                        c.name().as_deref() == Some("Status")
                    })
                    .unwrap();
                let content = preview
                    .find_descendant_of_type_where::<ContentControl>(false, |c| {
                        c.name().as_deref() == Some("ManagedPreview")
                    })
                    .unwrap();
                assert_eq!(status.text().as_deref(), Some("Dock"));
                assert_eq!(content.width(), 120.0);
                assert_eq!(content.height(), 80.0);
                assert!(content.is_visible());
                assert!(preview.control_recycling().is_some());
                assert!(preview.content_template().is_some());
                let rendered =
                    ferroui_controls::Control::from_boxed(&content.content().unwrap()).unwrap();
                assert_eq!(Some(rendered), preview.preview_content());
                preview.set_status(Some("Float".into()));
                preview.set_show_content(false);
                preview.set_preview_content_width(250.0);
                preview.set_preview_content_height(180.0);
                assert_eq!(status.text().as_deref(), Some("Float"));
                assert!(!content.is_visible());
                assert_eq!(content.width(), 250.0);
                assert_eq!(content.height(), 180.0);
                preview.set_preview_content(None);
                assert!(content.content().is_none());
                let host = DragPreviewWindow::new();
                let theme = panel.find_resource(&ferroui_base::controls::ResourceKey::from(
                    DragPreviewWindow::TYPE,
                ));
                let theme = ferroui_base::metadata::from_markup_value::<
                    Ref<ferroui_base::styling::ControlTheme>,
                >(&theme)
                .unwrap();
                host.set_theme(Some(theme));
                host.apply_styling();
                assert_eq!(
                    host.window_decorations(),
                    ferroui_controls::WindowDecorations::None
                );
                assert!(!host.show_in_taskbar());
                assert!(!host.can_resize());
                assert!(!host.show_activated());
                assert!(host.background().is_none());
                assert_eq!(
                    &*host.transparency_level_hint(),
                    &[ferroui_controls::WindowTransparencyLevel::transparent()]
                );
                assert_eq!(
                    host.size_to_content(),
                    ferroui_controls::SizeToContent::WIDTH_AND_HEIGHT
                );
                assert!(host.topmost());
                assert!(!host.is_hit_test_visible());
                host.close();
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

#[test]
fn complete_source_preview_theme_tracks_original_title_status_and_recycled_or_managed_content() {
    use ferrodock_model::core::IDockable;
    use ferrodock_model_reactiveui::controls::Document;
    use ferroui_base::controls::ResourceKey;
    use ferroui_controls::{shapes::Path, templates::FuncDataTemplate, Border, Control};
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/DragPreviewThemeConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let preview = panel
                    .children()
                    .get(0)
                    .cast::<DragPreviewControl>()
                    .unwrap();
                let built = Rc::new(std::cell::Cell::new(0));
                let builds = built.clone();
                preview.data_templates().add(FuncDataTemplate::new(
                    |v| v.is_some(),
                    move |_, _| {
                        builds.set(builds.get() + 1);
                        Some(Border::new().upcast())
                    },
                    true,
                ));
                let model = Document::new();
                model.set_title("Original title".into());
                preview.set_data_context(Some(
                    ferrodock_ferro::model_projection::to_dockable_value(model.clone()),
                ));
                let window = Window::new();
                window.set_width(640.0);
                window.set_height(480.0);
                window.set_content(Some(Rc::new(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let title = preview
                    .find_descendant_of_type_where::<TextBlock>(false, |t| {
                        t.text().as_deref() == Some("Original title")
                    })
                    .unwrap();
                model.set_title("Live title".into());
                assert_eq!(title.text().as_deref(), Some("Live title"));
                let status = preview
                    .find_descendant_of_type_where::<TextBlock>(false, |c| {
                        c.name().as_deref() == Some("PART_StatusText")
                    })
                    .unwrap();
                let icon = preview
                    .find_descendant_of_type_where::<Path>(false, |c| {
                        c.name().as_deref() == Some("PART_StatusIcon")
                    })
                    .unwrap();
                assert_eq!(status.text().as_deref(), Some("Dock"));
                assert!(icon.data().is_some());
                assert!(icon.is_visible());
                panel.resources().set(
                    ResourceKey::from("DragPreviewControlDockString"),
                    Some(Rc::new("Dock translated".to_owned())),
                );
                assert_eq!(status.text().as_deref(), Some("Dock translated"));
                preview.set_status(Some("Float".into()));
                assert_eq!(status.text().as_deref(), Some("Float"));
                assert!(icon.data().is_some());
                preview.set_status(Some("None".into()));
                assert_eq!(status.text().as_deref(), Some("None"));
                assert!(!icon.is_visible());
                preview.set_status(Some("Custom".into()));
                assert_eq!(status.text().as_deref(), Some("Custom"));
                assert!(icon.is_visible());
                let fallback = preview
                    .find_descendant_of_type_where::<ContentControl>(false, |c| {
                        c.name().as_deref() == Some("PART_ContentPresenter")
                    })
                    .unwrap();
                let managed = preview
                    .find_descendant_of_type_where::<ContentControl>(false, |c| {
                        c.name().as_deref() == Some("PART_PreviewContentPresenter")
                    })
                    .unwrap();
                let host = preview
                    .find_descendant_of_type_where::<Border>(false, |c| {
                        c.name().as_deref() == Some("PART_ContentHost")
                    })
                    .unwrap();
                assert!(fallback.is_visible());
                assert!(!managed.is_visible());
                assert!(host.is_visible());
                assert_eq!(built.get(), 1);
                assert_eq!(fallback.width(), 200.0);
                assert_eq!(fallback.height(), 100.0);
                let rendered = fallback.find_descendant_of_type::<Border>(false).unwrap();
                let direct = Border::new().upcast::<Control>();
                preview.set_preview_content(Some(direct.clone()));
                window.update_layout();
                assert!(managed.is_visible());
                assert!(!fallback.is_visible());
                assert_eq!(
                    Control::from_boxed(&managed.content().unwrap()),
                    Some(direct)
                );
                preview.set_preview_content(None);
                window.update_layout();
                assert!(!managed.is_visible());
                assert!(fallback.is_visible());
                assert_eq!(built.get(), 1);
                assert_eq!(
                    fallback.find_descendant_of_type::<Border>(false),
                    Some(rendered)
                );
                preview.set_show_content(false);
                assert!(!host.is_visible());
                preview.set_show_content(true);
                assert!(host.is_visible());
                preview.set_preview_content_width(250.0);
                preview.set_preview_content_height(180.0);
                assert_eq!(managed.width(), 250.0);
                assert_eq!(fallback.height(), 180.0);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
