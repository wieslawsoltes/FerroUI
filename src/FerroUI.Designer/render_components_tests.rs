//! Real native library, typed definition editors and WYSIWYG template scopes.
use super::*;
use crate::ComponentLibraryFilter;
use ferroui_controls::{ComboBox, Flyout, TextBox};
use ferroui_designer_core::NodeId;

const SOURCE: &str = include_str!("../../docs/designer/samples/ComponentsAndThemes.xaml");

fn resource(workspace: &DesignerWorkspace, key: &str) -> NodeId {
    workspace
        .session()
        .borrow()
        .component_catalog()
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some(key))
        .unwrap()
        .node
}
fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> ferroui_base::Ref<T> {
    // Definition rows and popup contents are built in code, outside the workspace's
    // XAML name scope. Traverse the actual native logical controls instead.
    root.get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing native component editor control {name}"))
}
fn click(workspace: &DesignerWorkspace, name: &str) {
    named::<Button>(&workspace.control(), name)
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn native_component_library_theme_editor_and_template_scope_render_authored_changes() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let clock = Rc::new(RenderClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
        now: Cell::new(0.0),
    });
    let mut services = TestServices::styled_window();
    services.global_clock = Some(clock.clone());
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native = MockWindowImpl::window(1600.0, 1040.0);
        native.max_auto_size_hint.set(Size::new(1600.0, 1040.0));
        let screen = PixelRect::new(0, 0, 1600, 1040);
        native.setup_feature::<dyn IScreenImpl>(MockScreenImpl::new(vec![mock_screen(
            1.0, screen, screen, true,
        )]));
        native.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native
    }));
    services.render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    services.font_manager_impl = FerroLocator::current().get_service::<dyn IFontManagerImpl>();
    services.text_shaper_impl = FerroLocator::current().get_service::<dyn ITextShaperImpl>();
    services.theme = Some(Rc::new(|| {
        ferroui_themes_winui::WinUITheme::new().as_style()
    }));
    let app = UnitTestApplication::start(services);
    ferroui_themes_winui::register_types();
    if let Some(application) = ferroui_controls::Application::current() {
        application.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::dark()));
        FerroLocator::current_mutable()
            .bind::<dyn ferroui_base::styling::IThemeVariantHost>()
            .to_constant(application.as_theme_variant_host());
    }
    let workspace =
        DesignerWorkspace::with_uri(SOURCE, "file:///ComponentsAndThemes.xaml".into()).unwrap();
    workspace.set_artboard_size(800.0, 620.0).unwrap();
    workspace.set_timeline_visible(false);
    workspace.set_zoom(1.0);
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    let session = workspace.session();
    let instance = session
        .borrow()
        .document()
        .ast()
        .find_name("PublishAction")
        .unwrap();
    session.borrow_mut().select([instance]);
    workspace.refresh(true);
    let theme = resource(&workspace, "PrimaryAction");
    workspace.inspect_component(theme).unwrap();
    let library = capture(
        &workspace,
        &clock,
        &directory.join("designer-component-library.png"),
    );
    assert_eq!(session.borrow().selection(), &[instance]);
    workspace
        .inspect_component(resource(&workspace, "QuietAction"))
        .unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-theme-object-setter.png"),
    );
    let editor = named::<Grid>(&workspace.control(), "DefinitionProperty_Background");
    let more = editor.children().get(1).cast::<Button>().unwrap();
    let flyout = more.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.show_at(&more);
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    assert!(flyout.is_open());
    let body = Control::from_boxed(&flyout.content().unwrap()).unwrap();
    named::<ComboBox>(&body, "PropertyObjectNodes").set_selected_index(1);
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.clone().upcast());
    bitmap
        .save_to_file(
            directory
                .join("designer-property-object-workbench.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    flyout.hide();
    let accent = resource(&workspace, "BrandAccent");
    workspace.inspect_component(accent).unwrap();
    let brush = capture(
        &workspace,
        &clock,
        &directory.join("designer-resource-editor.png"),
    );
    assert_ne!(library, brush);
    let template = resource(&workspace, "ActionButtonTemplate");
    workspace.inspect_component(template).unwrap();
    named::<TextBox>(&workspace.control(), "DefinitionPreviewContent")
        .set_text(Some("Preview your component"));
    named::<TextBox>(&workspace.control(), "DefinitionPreviewWidth").set_text(Some("420"));
    named::<TextBox>(&workspace.control(), "DefinitionPreviewHeight").set_text(Some("180"));
    let source_before = session.borrow().document().source().to_owned();
    click(&workspace, "DefinitionEditCanvas");
    assert_eq!(
        session.borrow().document().source(),
        source_before,
        "Entering a preview never authors synthetic host XAML"
    );
    assert_eq!(session.borrow().definition_scope().unwrap().node, template);
    let frame = session
        .borrow()
        .document()
        .ast()
        .elements()
        .find(|node| node.xaml_name() == Some("ButtonFrame"))
        .unwrap()
        .id;
    session.borrow_mut().select([frame]);
    // Inspect the actual template rendering without the optional content-occupancy
    // helper covering the sample's centered text. Selection adorners remain visible.
    workspace.set_layout_helpers_visible(false);
    let scoped = capture(
        &workspace,
        &clock,
        &directory.join("designer-template-canvas.png"),
    );
    assert_ne!(brush, scoped);
    session
        .borrow_mut()
        .set_property(&[frame], "CornerRadius", "24")
        .unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-template-edited.png"),
    );
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(frame)
            .unwrap()
            .attribute("CornerRadius"),
        Some("24")
    );
    workspace.return_to_document().unwrap();
    workspace.set_layout_helpers_visible(true);
    assert_eq!(session.borrow().selection(), &[instance]);
    workspace.inspect_component(theme).unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-component-instances.png"),
    );
    workspace
        .register_component_document("library:///ProjectComponents.xaml", SOURCE)
        .unwrap();
    workspace
        .component_library()
        .unwrap()
        .set_filter(ComponentLibraryFilter::Templates);
    workspace
        .component_library()
        .unwrap()
        .set_query("ActionButtonTemplate");
    capture(
        &workspace,
        &clock,
        &directory.join("designer-definition-library-sources.png"),
    );
    window.close();
    drop(app);
    drop(locator);
}
