//! Native rendering evidence through FerroUI's Vello CPU backend and HarfBuzz shaping.
use crate::{
    AssetViewMode, DesignerWorkspace, NavigationPane, PreviewStatus, STARTER_XAML,
    WorkspaceViewMode,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Rect, Size};
use ferroui_controls::platform::{IScreenImpl, ITopLevelImpl};
use ferroui_controls::testing::{
    MockScreenImpl, MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
    mock_screen,
};
use ferroui_controls::{Button, Control, Grid, GridLength, Window};
use ferroui_designer_animation::{AnimationValue, Easing};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

// WinUI control transitions require the same global clock service as a running application.
// Keep theme transitions deterministic; document animation uses its own seekable native clocks.
struct RenderClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
    now: Cell<f64>,
}
impl RenderClock {
    fn pulse(&self) {
        let now = self.now.get() + 1.0;
        self.now.set(now);
        self.subject.on_next(TimeSpan::from_seconds(now));
    }
}
impl IObservable<TimeSpan> for RenderClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for RenderClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}
impl IGlobalClock for RenderClock {}

fn capture(workspace: &Rc<DesignerWorkspace>, clock: &RenderClock, path: &Path) -> Vec<u8> {
    capture_at(workspace, clock, path, Size::new(1600.0, 1040.0))
}

fn capture_at(
    workspace: &Rc<DesignerWorkspace>,
    clock: &RenderClock,
    path: &Path,
    size: Size,
) -> Vec<u8> {
    workspace.refresh(true);
    capture_current_at(workspace, clock, path, size)
}

// A pointer draft belongs to the live measured tree. Capture it without rebuilding the
// inspector or refreshing the selection while the pointer is captured.
fn capture_current_at(
    workspace: &Rc<DesignerWorkspace>,
    clock: &RenderClock,
    path: &Path,
    size: Size,
) -> Vec<u8> {
    assert!(
        matches!(
            workspace.session().borrow().preview_status(),
            PreviewStatus::Ready { .. }
        ),
        "Every rendered authoring step must have an accepted runtime preview: {:?}",
        workspace.session().borrow().preview_status()
    );
    workspace.synchronize_language_service().unwrap();
    let uri = workspace.session().borrow().document_uri().to_owned();
    let diagnostics = workspace
        .try_language_service()
        .unwrap()
        .diagnostics(&uri)
        .unwrap();
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic.severity > 2),
        "Valid native render fixtures must also be accepted by the source language service: {diagnostics:?}"
    );
    let control = workspace.control();
    control.measure(size);
    control.arrange(Rect::from_size(size));
    control.update_layout();
    // Let WinUI brush/opacity transitions reach their final values without wall-clock sleeps.
    // The first tick establishes a transition origin; subsequent ticks settle styles created
    // by layout and queued control events. Document animations retain their explicit seek time.
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        clock.pulse();
        control.update_layout();
    }
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        control.bounds().size(),
        size,
        "The attached workspace must occupy the full render surface"
    );
    let width = size.width as i32;
    let height = size.height as i32;
    let bitmap = RenderTargetBitmap::new(PixelSize::new(width, height));
    bitmap.render(&control.upcast());
    let mut pixels = vec![0; width as usize * height as usize * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, width, height), &mut pixels, width * 4);
    let distinct: std::collections::BTreeSet<_> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| (p[0], p[1], p[2]))
        .collect();
    assert!(
        distinct.len() > 128,
        "Native workspace should contain text, controls and antialiasing, not a blank bitmap"
    );
    bitmap
        .save_to_file(
            path.to_str().unwrap(),
            &PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    bitmap.dispose();
    pixels
}

#[path = "render_layout_helpers_tests.rs"]
mod layout_helpers;

#[path = "render_components_tests.rs"]
mod components;
#[path = "render_design_workflows_tests.rs"]
mod design_workflows;
#[path = "render_navigation_tests.rs"]
mod navigation;
#[path = "render_icon_tests.rs"]
mod icons;
#[path = "render_binding_tests.rs"]
mod bindings;
#[path = "render_property_inspector_tests.rs"]
mod property_inspector;

#[test]
fn native_workspace_renders_design_manipulation_and_animation() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let mut services = TestServices::styled_window();
    let clock = Rc::new(RenderClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
        now: Cell::new(0.0),
    });
    services.global_clock = Some(clock.clone());
    services.windowing_platform = Some(MockWindowingPlatform::with_window_impl(|| {
        let native_window = MockWindowImpl::window(1600.0, 1040.0);
        native_window
            .max_auto_size_hint
            .set(Size::new(1600.0, 1040.0));
        let screen = PixelRect::new(0, 0, 1600, 1040);
        native_window.setup_feature::<dyn IScreenImpl>(MockScreenImpl::new(vec![mock_screen(
            1.0, screen, screen, true,
        )]));
        native_window.setup_resize(|window, size, reason| {
            if window.client_size.get() != size {
                window.client_size.set(size);
                if let Some(resized) = ITopLevelImpl::resized(window) {
                    resized(size, reason);
                }
            }
        });
        native_window
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
        // UnitTestApplication installs global styles but does not register the application
        // theme host. Mirror the real host so top levels inherit the requested dark variant.
        FerroLocator::current_mutable()
            .bind::<dyn ferroui_base::styling::IThemeVariantHost>()
            .to_constant(application.as_theme_variant_host());
    }
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    assert_eq!(
        window.actual_theme_variant(),
        Some(ferroui_base::styling::ThemeVariant::dark())
    );
    assert_eq!(
        workspace
            .control()
            .get_control::<Control>("PreviewThemeScope")
            .actual_theme_variant(),
        Some(ferroui_base::styling::ThemeVariant::light())
    );
    let session = workspace.session();
    let target = session.borrow().document().ast().find_name("Card").unwrap();
    session.borrow_mut().select([target]);
    assert!(matches!(
        session.borrow().preview_status(),
        PreviewStatus::Ready { .. }
    ));
    let directory = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroui-designer-renders"));
    std::fs::create_dir_all(&directory).unwrap();
    let before = capture(
        &workspace,
        &clock,
        &directory.join("designer-workspace.png"),
    );
    assert_eq!(workspace.view_mode(), WorkspaceViewMode::Design);
    let canvas = workspace.control().get_control::<Control>("ArtboardScroll");
    assert!(
        canvas.bounds().height > 580.0,
        "Design mode must leave room for the artboard"
    );
    workspace.set_navigation_pane(NavigationPane::Assets);
    let assets = capture(&workspace, &clock, &directory.join("designer-assets.png"));
    assert_ne!(
        before, assets,
        "The assets tab must expose the native toolbox"
    );
    let toolbox = workspace.asset_toolbox().unwrap();
    toolbox.set_view_mode(AssetViewMode::List);
    capture(
        &workspace,
        &clock,
        &directory.join("designer-assets-list.png"),
    );
    toolbox.set_view_mode(AssetViewMode::Grid);
    workspace.set_navigation_pane(NavigationPane::Layers);
    workspace.set_canvas_focused(true);
    capture(
        &workspace,
        &clock,
        &directory.join("designer-canvas-focus.png"),
    );
    workspace.set_canvas_focused(false);
    workspace.set_view_mode(WorkspaceViewMode::Split);
    window.update_layout();
    workspace.fit_artboard().unwrap();
    let split = capture(&workspace, &clock, &directory.join("designer-split.png"));
    assert_ne!(before, split);
    assert!(
        workspace
            .control()
            .get_control::<Control>("SourcePane")
            .bounds()
            .height
            >= 200.0
    );
    workspace.set_view_mode(WorkspaceViewMode::Design);
    workspace.set_zoom(1.0);
    let layout = session
        .borrow()
        .document()
        .ast()
        .elements()
        .find(|node| node.local_name() == "Grid")
        .unwrap()
        .id;
    session.borrow_mut().select([layout]);
    capture(
        &workspace,
        &clock,
        &directory.join("designer-grid-layout.png"),
    );
    assert_eq!(workspace.grid_track_editors().len(), 2);
    session.borrow_mut().select([target]);
    for (button, filename) in [
        ("StatesMode", "designer-states.png"),
        ("BehaviorsMode", "designer-behaviors.png"),
    ] {
        workspace
            .control()
            .get_control::<Button>(button)
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        capture(&workspace, &clock, &directory.join(filename));
    }
    workspace
        .control()
        .get_control::<Button>("CloseAuthoring")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    session
        .borrow_mut()
        .set_property(&[target], "Canvas.Left", "96")
        .unwrap();
    let after = capture(
        &workspace,
        &clock,
        &directory.join("designer-manipulated.png"),
    );
    assert_ne!(
        before, after,
        "A source transaction must change actual native rendered pixels"
    );
    session
        .borrow_mut()
        .record_keyframe(target, "Width", AnimationValue::Number(656.0))
        .unwrap();
    session.borrow_mut().seek(3.0).unwrap();
    session
        .borrow_mut()
        .record_keyframe(target, "Width", AnimationValue::Number(560.0))
        .unwrap();
    session.borrow_mut().seek(1.5).unwrap();
    let animated = capture(
        &workspace,
        &clock,
        &directory.join("designer-animation.png"),
    );
    assert_ne!(
        after, animated,
        "Animation scrubbing must change actual native rendered pixels"
    );
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Width"),
        Some("656"),
        "Preview layers must not overwrite authored values"
    );
    let authored = session.borrow().document().source().to_owned();
    workspace.set_timeline_frame_rate(Some(30));
    workspace
        .select_keyframes(&[(target, "Width".into(), 0.0), (target, "Width".into(), 3.0)])
        .unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-timeline-selection.png"),
    );
    assert_eq!(
        session.borrow().document().source(),
        authored,
        "Timeline selection and frame-grid UI must remain view state"
    );
    window.set_width(1280.0);
    window.set_height(820.0);
    window.update_layout();
    workspace.fit_artboard().unwrap();
    capture_at(
        &workspace,
        &clock,
        &directory.join("designer-compact.png"),
        Size::new(1280.0, 820.0),
    );
    assert!(
        workspace.zoom() < 1.0,
        "Fit must use the smaller measured viewport"
    );
    // A real multi-property program exposes the professional motion workflow: lanes,
    // fixed ruler, selected key fields and the sampled easing preview in one native view.
    window.set_width(1600.0);
    window.set_height(1040.0);
    workspace
        .control()
        .cast::<Grid>()
        .unwrap()
        .row_definitions()
        .get(4)
        .set_height(GridLength::from_pixels(264.0));
    for (property, start, end) in [("Height", 352.0, 320.0), ("Opacity", 1.0, 0.85)] {
        session.borrow_mut().seek(0.0).unwrap();
        session
            .borrow_mut()
            .record_keyframe(target, property, AnimationValue::Number(start))
            .unwrap();
        session.borrow_mut().seek(3.0).unwrap();
        session
            .borrow_mut()
            .record_keyframe(target, property, AnimationValue::Number(end))
            .unwrap();
    }
    session
        .borrow_mut()
        .set_keyframe_easing(
            target,
            "Width",
            3.0,
            Easing::CubicBezier {
                x1: 0.2,
                y1: 0.0,
                x2: 0.2,
                y2: 1.0,
            },
        )
        .unwrap();
    workspace
        .select_keyframes(&[(target, "Width".into(), 3.0)])
        .unwrap();
    window.update_layout();
    workspace.fit_artboard().unwrap();
    capture(
        &workspace,
        &clock,
        &directory.join("designer-motion-editor.png"),
    );
    workspace
        .control()
        .get_control::<Button>("AnimationSettings")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    capture(
        &workspace,
        &clock,
        &directory.join("designer-animation-settings.png"),
    );
    workspace
        .control()
        .get_control::<Button>("CloseAuthoring")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    workspace.show_sharing();
    capture(&workspace, &clock, &directory.join("designer-sharing.png"));
    println!("Designer native render evidence: {}", directory.display());
    window.close();
    drop(window);
    drop(session);
    drop(workspace);
    // A real application commonly authors a Window root. Verify that its content survives
    // attachment to the design artboard, rather than testing only a detached logical tree.
    let workspace=DesignerWorkspace::new(r##"<Window xmlns="https://github.com/ferroui" Width="640" Height="360" Background="#FFECF4FF"><Canvas><Border Name="WindowTarget" Canvas.Left="32" Canvas.Top="48" Width="160" Height="90" Background="#FFEA427D"/><TextBlock Text="Window document preview" FontSize="22" Canvas.Left="32" Canvas.Top="170"/></Canvas></Window>"##).unwrap();
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let pixels = capture(
        &workspace,
        &clock,
        &directory.join("designer-window-root.png"),
    );
    let accent = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[1] < 100 && ((p[0] > 210 && p[2] > 100) || (p[2] > 210 && p[0] > 100)))
        .count();
    assert!(
        accent > 10000,
        "A Window-root preview must actually paint its authored 160x90 accent rectangle ({accent} pixels)"
    );
    window.close();
    drop(window);
    drop(workspace);
    drop(app);
    locator.dispose();
}
