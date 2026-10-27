//! Native evidence for design data, keyboard navigation, local layers and responsive previews.
use super::*;
use ferroui_controls::{Flyout, ListBox, TextBox};

const SOURCE: &str = include_str!("../../docs/designer/samples/DesignWorkflows.xaml");

fn open(
    workspace: &DesignerWorkspace,
    window: &Window,
    clock: &RenderClock,
    name: &str,
) -> ferroui_base::Ref<Flyout> {
    let button = workspace.control().get_control::<Button>(name);
    let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.show_at(&button);
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    assert!(flyout.is_open());
    flyout
}

fn popup_capture(window: &Window, clock: &RenderClock, path: &Path) {
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.to_ref().upcast());
    bitmap
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
}

#[test]
fn native_design_workflows_render_data_focus_layers_and_preview_environment() {
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
        DesignerWorkspace::with_uri(SOURCE, "file:///DesignWorkflows.xaml".into()).unwrap();
    workspace.set_artboard_size(480.0, 680.0).unwrap();
    workspace.set_timeline_visible(false);
    workspace.set_layout_helpers_visible(false);
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
    let email = session
        .borrow()
        .document()
        .ast()
        .find_name("Email")
        .unwrap();
    let card = session
        .borrow()
        .document()
        .ast()
        .find_name("ProjectCard")
        .unwrap();
    let preferences = session
        .borrow()
        .document()
        .ast()
        .find_name("Preferences")
        .unwrap();
    let list = session
        .borrow()
        .document()
        .ast()
        .find_name("Projects")
        .unwrap();
    session.borrow_mut().select([email]);
    workspace.refresh(true);
    assert_eq!(
        session
            .borrow()
            .preview
            .visual(email)
            .unwrap()
            .cast::<TextBox>()
            .unwrap()
            .text()
            .as_deref(),
        Some("alex@example.com")
    );
    assert_eq!(
        session
            .borrow()
            .preview
            .visual(list)
            .unwrap()
            .cast::<ListBox>()
            .unwrap()
            .items()
            .count(),
        4
    );
    let before = capture(
        &workspace,
        &clock,
        &directory.join("designer-design-data-preview.png"),
    );
    let flyout = open(&workspace, &window, &clock, "DesignData");
    popup_capture(
        &window,
        &clock,
        &directory.join("designer-design-data-editor.png"),
    );
    flyout.hide();
    let flyout = open(&workspace, &window, &clock, "FocusOrder");
    popup_capture(&window, &clock, &directory.join("designer-focus-order.png"));
    flyout.hide();
    session.borrow_mut().select([preferences]);
    session
        .borrow_mut()
        .set_layers_locked(&[card], true)
        .unwrap();
    session.borrow_mut().isolate_layers(&[preferences]).unwrap();
    let isolated = capture(
        &workspace,
        &clock,
        &directory.join("designer-layer-isolation.png"),
    );
    assert_ne!(before, isolated);
    session.borrow_mut().reset_layer_view();
    workspace.refresh(true);
    let flyout = open(&workspace, &window, &clock, "PreviewEnvironment");
    popup_capture(
        &window,
        &clock,
        &directory.join("designer-preview-environment.png"),
    );
    flyout.hide();
    workspace
        .set_preview_environment(crate::PreviewEnvironment {
            width: 844.0,
            height: 480.0,
            theme: crate::PreviewTheme::Dark,
            flow_direction: crate::PreviewFlowDirection::RightToLeft,
        })
        .unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-responsive-rtl-preview.png"),
    );
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert!(!session.borrow().can_undo());
    window.close();
    drop(workspace);
    drop(app);
    drop(locator);
}
