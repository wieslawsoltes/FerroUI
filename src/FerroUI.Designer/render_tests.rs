//! Native rendering evidence through FerroUI's Vello CPU backend and HarfBuzz shaping.
use crate::{DesignerWorkspace, PreviewStatus, STARTER_XAML};
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Rect, Size};
use ferroui_controls::platform::{IScreenImpl, ITopLevelImpl};
use ferroui_controls::testing::{
    mock_screen, MockScreenImpl, MockWindowImpl, MockWindowingPlatform, TestServices,
    UnitTestApplication,
};
use ferroui_controls::{Control, Window};
use ferroui_designer_animation::AnimationValue;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
use std::path::Path;
use std::rc::Rc;

fn capture(workspace: &Rc<DesignerWorkspace>, path: &Path) -> Vec<u8> {
    workspace.refresh(true);
    assert!(
        matches!(
            workspace.session().borrow().preview_status(),
            PreviewStatus::Ready { .. }
        ),
        "Every rendered authoring step must have an accepted runtime preview: {:?}",
        workspace.session().borrow().preview_status()
    );
    let control = workspace.control();
    let size = Size::new(1600.0, 1040.0);
    control.measure(size);
    control.arrange(Rect::from_size(size));
    control.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        control.bounds().size(),
        size,
        "The attached workspace must occupy the full render surface"
    );
    let bitmap = RenderTargetBitmap::new(PixelSize::new(1600, 1040));
    bitmap.render(&control.upcast());
    let mut pixels = vec![0; 1600 * 1040 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 1600, 1040), &mut pixels, 1600 * 4);
    let distinct: std::collections::BTreeSet<_> =
        pixels.chunks_exact(4).map(|p| (p[0], p[1], p[2])).collect();
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
        ferroui_themes_simple::SimpleTheme::new().as_style()
    }));
    let app = UnitTestApplication::start(services);
    ferroui_themes_simple::register_types();
    if let Some(application) = ferroui_controls::Application::current() {
        application.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::dark()));
    }
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    let window = Window::new();
    window.set_width(1600.0);
    window.set_height(1040.0);
    window.set_content(Some(Control::boxed(&workspace.control())));
    window.show();
    window.update_layout();
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
    let before = capture(&workspace, &directory.join("designer-workspace.png"));
    session
        .borrow_mut()
        .set_property(&[target], "Canvas.Left", "96")
        .unwrap();
    let after = capture(&workspace, &directory.join("designer-manipulated.png"));
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
    let animated = capture(&workspace, &directory.join("designer-animation.png"));
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
    window.set_content(Some(Control::boxed(&workspace.control())));
    window.show();
    window.update_layout();
    let pixels = capture(&workspace, &directory.join("designer-window-root.png"));
    let accent = pixels
        .chunks_exact(4)
        .filter(|p| p[1] < 100 && ((p[0] > 210 && p[2] > 100) || (p[2] > 210 && p[0] > 100)))
        .count();
    assert!(accent>10000,"A Window-root preview must actually paint its authored 160x90 accent rectangle ({accent} pixels)");
    window.close();
    drop(window);
    drop(workspace);
    drop(app);
    locator.dispose();
}
