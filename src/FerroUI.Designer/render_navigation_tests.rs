//! Render-transform navigation must agree with native pointer and visual coordinates.
use super::*;
use ferroui_base::input::{
    KeyModifiers, Pointer, PointerPointProperties, PointerType, PointerUpdateKind,
    PointerWheelEventArgs, RawInputModifiers,
};
use ferroui_base::{Point, Ref, Vector, Visual};
use ferroui_controls::Flyout;

fn wheel(
    workspace: &DesignerWorkspace,
    window: &Window,
    pointer: &Rc<Pointer>,
    position: Point,
    delta: Vector,
    modifiers: KeyModifiers,
) {
    let surface = workspace.design_surface();
    let event = PointerWheelEventArgs::new(
        surface.to_ref(),
        pointer.clone(),
        window,
        position,
        1,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        modifiers,
        delta,
    );
    surface.raise_event(&event);
    assert!(
        event.handled(),
        "Canvas navigation must consume its own wheel input"
    );
    window.update_layout();
}

fn center_in(visual: &Visual, destination: &Visual) -> Point {
    visual
        .translate_point(
            Point::new(visual.bounds().width / 2.0, visual.bounds().height / 2.0),
            destination,
        )
        .unwrap()
}

fn near(actual: Point, expected: Point) {
    assert!(
        (actual.x - expected.x).abs() < 0.05 && (actual.y - expected.y).abs() < 0.05,
        "Native visual coordinate {actual:?} must preserve pointer anchor {expected:?}"
    );
}

#[test]
fn native_canvas_navigation_renders_and_preserves_pointer_anchor_and_source_drafts() {
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
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    workspace.set_timeline_visible(false);
    workspace.set_layout_helpers_visible(false);
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    workspace.fit_artboard().unwrap();
    let session = workspace.session();
    let card_id = session.borrow().document().ast().find_name("Card").unwrap();
    session.borrow_mut().select([card_id]);
    workspace.refresh(true);
    window.update_layout();
    let card: Ref<Visual> = session.borrow().preview.visual(card_id).unwrap();
    let artboard = workspace.control().get_control::<Control>("Artboard");
    let viewport = workspace.control().get_control::<Control>("ArtboardScroll");
    let authored_bounds = card.bounds();
    let authored_center = center_in(&card, &artboard);
    let source = session.borrow().document().source().to_owned();
    let ast = session.borrow().document().ast_snapshot();
    let revision = session.borrow().document().revision();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let anchor = center_in(&card, &window);
    let initial_zoom = workspace.zoom();
    wheel(
        &workspace,
        &window,
        &pointer,
        anchor,
        Vector::new(0.0, 0.5),
        KeyModifiers::CONTROL,
    );
    assert!(workspace.zoom() > initial_zoom);
    near(center_in(&card, &window), anchor);
    near(center_in(&card, &artboard), authored_center);
    assert_eq!(
        card.bounds(),
        authored_bounds,
        "Zoom must not relayout authored controls"
    );

    let zoom = workspace.zoom();
    wheel(
        &workspace,
        &window,
        &pointer,
        anchor,
        Vector::new(-0.375, 0.25),
        KeyModifiers::NONE,
    );
    assert_eq!(workspace.zoom(), zoom);
    let panned = center_in(&card, &window);
    assert!(
        panned.x < anchor.x && panned.y > anchor.y,
        "Fractional wheel input must pan both native axes: {anchor:?} -> {panned:?}"
    );
    near(center_in(&card, &artboard), authored_center);

    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    let panned_pixels = capture_current_at(
        &workspace,
        &clock,
        &directory.join("designer-canvas-navigation.png"),
        Size::new(1600.0, 1040.0),
    );
    workspace.fit_selection().unwrap();
    window.update_layout();
    let fitted_pixels = capture_current_at(
        &workspace,
        &clock,
        &directory.join("designer-canvas-fit-selection.png"),
        Size::new(1600.0, 1040.0),
    );
    assert_ne!(
        panned_pixels, fitted_pixels,
        "Native fit selection must change rendered pixels"
    );
    let top_left = card
        .translate_point(Point::new(0.0, 0.0), &viewport)
        .unwrap();
    let bottom_right = card
        .translate_point(
            Point::new(card.bounds().width, card.bounds().height),
            &viewport,
        )
        .unwrap();
    assert!(top_left.x >= 0.0 && top_left.y >= 0.0);
    assert!(
        bottom_right.x <= viewport.bounds().width && bottom_right.y <= viewport.bounds().height
    );

    let options = workspace.control().get_control::<Button>("ZoomOptions");
    let flyout = options.flyout().unwrap().cast::<Flyout>().unwrap();
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.show_at(&options);
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        window.update_layout();
    }
    assert!(flyout.is_open());
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&window.to_ref().upcast());
    bitmap
        .save_to_file(
            directory
                .join("designer-canvas-zoom-options.png")
                .to_str()
                .unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    flyout.hide();

    // Toolbar commands and wheel navigation must never apply or discard a dirty XAML draft.
    workspace
        .source_editor()
        .set_text(Some("<Canvas unfinished"));
    let generation = session.borrow().generation();
    let anchor = center_in(&card, &window);
    wheel(
        &workspace,
        &window,
        &pointer,
        anchor,
        Vector::new(0.0, -0.25),
        KeyModifiers::CONTROL,
    );
    for name in ["ZoomOut", "ZoomIn", "ZoomReset", "ZoomFit"] {
        workspace
            .control()
            .get_control::<Button>(name)
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    }
    assert_eq!(
        workspace.source_editor().text().as_deref(),
        Some("<Canvas unfinished")
    );
    assert_eq!(session.borrow().generation(), generation);
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().document().source(), source);
    assert!(std::sync::Arc::ptr_eq(
        &session.borrow().document().ast_snapshot(),
        &ast
    ));
    assert_eq!(session.borrow().selection(), &[card_id]);
    assert!(!session.borrow().can_undo());
    window.close();
    drop(flyout);
    drop(options);
    drop(window);
    drop(card);
    drop(artboard);
    drop(viewport);
    drop(pointer);
    drop(session);
    drop(workspace);
    drop(app);
    locator.dispose();
}
