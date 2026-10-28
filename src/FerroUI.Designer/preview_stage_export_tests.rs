use super::*;

#[test]
fn screenshot_limits_validate_before_allocating_native_pixels() {
    let normal = PreviewStageExportOptions::default();
    assert_eq!(
        normal.pixel_size(Size::new(4096.0, 4096.0)).unwrap(),
        PixelSize::new(4096, 4096)
    );
    assert!(normal.pixel_size(Size::new(4097.0, 4097.0)).is_err());
    assert!(normal.pixel_size(Size::new(16_384.0, 16_384.0)).is_err());
    assert!(PreviewStageExportOptions { scale: 4.0 }
        .pixel_size(Size::new(16_384.0, 1.0))
        .is_err());
    for scale in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.249, 4.001] {
        assert!(PreviewStageExportOptions { scale }
            .pixel_size(Size::new(100.0, 100.0))
            .is_err());
    }
    for size in [
        Size::new(0.0, 10.0),
        Size::new(10.0, f64::INFINITY),
        Size::new(f64::NAN, 10.0),
        Size::new(10.0, 16_385.0),
    ] {
        assert!(normal.pixel_size(size).is_err());
    }
    assert_eq!(
        PreviewStageExportOptions { scale: 1.5 }
            .pixel_size(Size::new(101.5, 40.25))
            .unwrap(),
        PixelSize::new(153, 61),
        "Fractional logical dimensions need the complete enclosing pixel extent"
    );
    assert_eq!(
        PreviewStageExportOptions { scale: 0.25 }
            .pixel_size(Size::new(1.0, 1.0))
            .unwrap(),
        PixelSize::new(1, 1)
    );
}

#[test]
fn screenshots_require_an_arranged_frame_and_do_not_fix_layout() {
    use ferroui_base::Rect;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::Border;

    let _app = UnitTestApplication::start(TestServices::default());
    let control = Border::new().upcast::<Control>();
    assert!(capture_preview_stage_png(&control, Default::default())
        .unwrap_err()
        .contains("Arrange the preview"));
    control.measure(Size::new(80.0, 40.0));
    control.arrange(Rect::new(0.0, 0.0, 80.0, 40.0));
    assert!(capture_preview_stage_png(&control, Default::default())
        .unwrap_err()
        .contains("no native rendering backend"));
    control.invalidate_measure();
    let before = control.bounds();
    assert!(capture_preview_stage_png(&control, Default::default())
        .unwrap_err()
        .contains("layout to settle"));
    assert_eq!(control.bounds(), before);
    assert!(!control.is_measure_valid());
}

#[test]
fn screenshots_reject_pending_descendant_layout_without_measuring_the_graph() {
    use ferroui_base::Rect;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::{Border, Grid};

    let _app = UnitTestApplication::start(TestServices::default());
    let root = Grid::new();
    let child = Border::new();
    root.children().add(child.clone().upcast::<Control>());
    root.measure(Size::new(80.0, 40.0));
    root.arrange(Rect::new(0.0, 0.0, 80.0, 40.0));
    child.invalidate_measure();
    assert!(
        root.is_measure_valid(),
        "Descendants can queue their own measure pass"
    );
    assert!(
        capture_preview_stage_png(&root.clone().upcast(), Default::default())
            .unwrap_err()
            .contains("layout to settle")
    );
    assert!(
        !child.is_measure_valid(),
        "Screenshot capture must not run pending layout"
    );
    child.set_is_visible(false);
    root.measure(Size::new(80.0, 40.0));
    root.arrange(Rect::new(0.0, 0.0, 80.0, 40.0));
    assert!(validate_visible_layout(&root.upcast()).is_ok());
}

#[cfg(feature = "render-smoke")]
#[test]
fn screenshot_renders_native_content_at_its_own_density_without_ancestor_scale_or_chrome() {
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::media::{SolidColorBrush, Stretch};
    use ferroui_base::{PixelRect, Rect, Thickness};
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::{Border, Grid, TextBox, Viewbox};
    use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};

    let _locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    let services = TestServices {
        render_interface: FerroLocator::current().get_service::<dyn IPlatformRenderInterface>(),
        ..Default::default()
    };
    let _app = UnitTestApplication::start(services);
    let stage = Grid::new();
    stage.set_width(80.0);
    stage.set_height(40.0);
    stage.set_background(Some(SolidColorBrush::parse("#FFFF0000").unwrap().into()));
    let right = Border::new();
    right.set_width(40.0);
    right.set_horizontal_alignment(HorizontalAlignment::Right);
    right.set_vertical_alignment(VerticalAlignment::Stretch);
    right.set_background(Some(SolidColorBrush::parse("#FF0000FF").unwrap().into()));
    stage.children().add(right.clone().upcast::<Control>());
    // Native input state belongs to this graph. Screenshot capture must neither
    // replace it nor invoke layout to create another preview.
    let input = TextBox::new();
    input.set_is_visible(false);
    input.set_text(Some("Independent live input"));
    stage.children().add(input.clone().upcast::<Control>());
    let viewbox = Viewbox::new();
    viewbox.set_stretch(Stretch::Fill);
    viewbox.set_width(160.0);
    viewbox.set_height(80.0);
    viewbox.set_child(Some(stage.clone().upcast::<Control>()));
    let chrome = Border::new();
    chrome.set_background(Some(SolidColorBrush::parse("#FF00FF00").unwrap().into()));
    chrome.set_padding(Thickness::uniform(20.0));
    chrome.set_child(Some(viewbox.clone().upcast::<Control>()));
    chrome.measure(Size::new(200.0, 120.0));
    chrome.arrange(Rect::new(0.0, 0.0, 200.0, 120.0));
    let stage_control = stage.clone().upcast::<Control>();
    let original_bounds = stage_control.bounds();
    let original_parent = stage_control.visual_parent();
    let original_children = stage.children().count();
    assert_eq!(original_bounds.size(), Size::new(80.0, 40.0));

    for scale in [0.5, 1.0, 2.0, 4.0] {
        let captured =
            capture_preview_stage_png(&stage_control, PreviewStageExportOptions { scale }).unwrap();
        assert_eq!(&captured.png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(captured.logical_width, 80.0);
        assert_eq!(captured.logical_height, 40.0);
        assert_eq!(captured.pixel_width, (80.0 * scale) as u32);
        assert_eq!(captured.pixel_height, (40.0 * scale) as u32);
        assert_eq!(captured.scale, scale);
        let bitmap = Bitmap::from_stream(&mut std::io::Cursor::new(&captured.png)).unwrap();
        let size = bitmap.pixel_size();
        assert_eq!(size.width as u32, captured.pixel_width);
        assert_eq!(size.height as u32, captured.pixel_height);
        let mut rgba = vec![0; size.width as usize * size.height as usize * 4];
        bitmap.copy_pixels(
            PixelRect::new(0, 0, size.width, size.height),
            &mut rgba,
            size.width * 4,
        );
        let at = |x: i32, y: i32| {
            let offset = (y as usize * size.width as usize + x as usize) * 4;
            rgba[offset..offset + 4].to_vec()
        };
        assert_eq!(at(2, 2), [255, 0, 0, 255]);
        assert_eq!(at(size.width - 3, size.height - 3), [0, 0, 255, 255]);
        assert_eq!(at(size.width / 4, size.height / 2), [255, 0, 0, 255]);
        assert_eq!(at(3 * size.width / 4, size.height / 2), [0, 0, 255, 255]);
        bitmap.dispose();
        assert_eq!(stage_control.bounds(), original_bounds);
        assert_eq!(stage_control.visual_parent(), original_parent);
        assert_eq!(stage.children().count(), original_children);
        assert_eq!(input.text().as_deref(), Some("Independent live input"));
        assert!(stage_control.is_measure_valid() && stage_control.is_arrange_valid());
    }
}

#[cfg(feature = "render-smoke")]
mod changing_renderer {
    use super::*;
    use ferroui_base::input::InputElementImpl;
    use ferroui_base::interactivity::InteractiveImpl;
    use ferroui_base::layout::LayoutableImpl;
    use ferroui_base::media::DrawingContext;
    use ferroui_base::{
        ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, StyledElementImpl,
        VisualImpl, VisualImplExt,
    };
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::{Border, ControlImpl};
    use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};

    #[repr(C)]
    struct InvalidatingRenderer {
        base: Border,
    }
    ferro_class!(InvalidatingRenderer: Border);
    ferro_impl_classes!(InvalidatingRenderer: FerroObjectImpl, StyledElementImpl, InteractiveImpl, InputElementImpl, LayoutableImpl, ControlImpl);
    impl VisualImpl for InvalidatingRenderer {
        fn render(this: &Self, context: &mut DrawingContext<'_>) {
            Self::parent_render(this, context);
            this.child().unwrap().invalidate_measure();
        }
    }

    #[test]
    fn screenshot_rejects_descendant_layout_invalidated_by_native_rendering() {
        let _locator = FerroLocator::enter_scope();
        VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
            VelloRenderingMode::Cpu,
        ));
        let services = TestServices {
            render_interface: FerroLocator::current().get_service::<dyn IPlatformRenderInterface>(),
            ..Default::default()
        };
        let _app = UnitTestApplication::start(services);
        let root = instantiate(InvalidatingRenderer {
            base: Border::construct(),
        });
        let child = Border::new();
        root.set_child(Some(child.clone().upcast::<Control>()));
        root.measure(Size::new(80.0, 40.0));
        root.arrange(Rect::new(0.0, 0.0, 80.0, 40.0));
        let error =
            capture_preview_stage_png(&root.clone().upcast(), Default::default()).unwrap_err();
        assert!(error.contains("layout to settle"), "{error}");
        assert!(
            root.is_measure_valid(),
            "Only the descendant changed during rendering"
        );
        assert!(!child.is_measure_valid());
    }
}
