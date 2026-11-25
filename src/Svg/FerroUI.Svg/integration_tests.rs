use crate::*;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Colors, DrawingContext, PlatformDrawingContext};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IPlatformRenderInterface, RenderTargetSceneInfo,
};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Size};
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn cropped_vector_bitmaps_preserve_draw_opacity_and_intrinsic_pixel_alpha() {
    use base64::Engine;

    fn embedded_image(svg: &str) -> skia_safe::Image {
        let xml = roxmltree::Document::parse(svg).unwrap();
        let href = xml
            .descendants()
            .find(|node| node.has_tag_name("image"))
            .unwrap()
            .attributes()
            .find(|attribute| attribute.name() == "href")
            .unwrap()
            .value();
        let png = base64::engine::general_purpose::STANDARD
            .decode(href.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap();
        skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&png))
            .unwrap()
            .make_raster_image(None, skia_safe::image::CachingHint::Allow)
            .unwrap()
    }

    for draw_opacity in [1.0, 0.85, 0.5] {
        let mut source = skia_safe::surfaces::raster_n32_premul((8, 4)).unwrap();
        source.canvas().clear(skia_safe::Color::RED);
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::from_argb(128, 0, 0, 255));
        paint.set_blend_mode(skia_safe::BlendMode::Src);
        source
            .canvas()
            .draw_rect(skia_safe::Rect::from_xywh(4.0, 0.0, 4.0, 4.0), &paint);
        let bitmap = ferroui_skia::ImmutableBitmap::from_image(source.image_snapshot(), None);
        let mut document = SvgDocument::new(Size::new(40.0, 24.0)).unwrap();
        document
            .record_layer(SvgLayerOptions::new("image", "Cropped bitmap"), |context| {
                context.draw_bitmap(
                    &bitmap,
                    draw_opacity,
                    Rect::new(1.0, 1.0, 6.0, 2.0),
                    Rect::new(8.0, 8.0, 24.0, 8.0),
                );
            })
            .unwrap();
        drop(bitmap);
        drop(source);

        let vector = document
            .export(&SvgExportOptions {
                fidelity: SvgFidelity::VectorOnly,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(1, vector.report.vector_layers);
        assert_eq!(0, vector.report.raster_layers);
        assert_eq!(1, vector.report.embedded_images);
        let xml = roxmltree::Document::parse(&vector.svg).unwrap();
        let instance = xml
            .descendants()
            .find(|node| node.has_tag_name("use"))
            .unwrap();
        assert_eq!(None, instance.attribute("fill-opacity"));
        let opacity = instance
            .attribute("opacity")
            .map(|value| value.parse::<f64>().unwrap())
            .unwrap_or(1.0);
        // FerroUI's native draw_bitmap quantizes its paint alpha to eight bits.
        // The SVG must apply that same multiplier to the retained source PNG.
        let native_alpha = (draw_opacity * 255.0) as u8;
        assert!((opacity - f64::from(native_alpha) / 255.0).abs() < 1e-7);

        let source = embedded_image(&vector.svg);
        let source_pixels = source.peek_pixels().unwrap();
        assert_eq!((8, 4), (source_pixels.width(), source_pixels.height()));
        assert_eq!(255, source_pixels.get_color((2, 2)).a());
        assert_eq!(128, source_pixels.get_color((6, 2)).a());
        let raster = document
            .export(&SvgExportOptions {
                fidelity: SvgFidelity::Raster,
                raster_scale: 1.0,
                ..Default::default()
            })
            .unwrap();
        let reference = embedded_image(&raster.svg);
        let reference_pixels = reference.peek_pixels().unwrap();
        assert_eq!(
            (40, 24),
            (reference_pixels.width(), reference_pixels.height())
        );
        assert_eq!(0, reference_pixels.get_color((4, 12)).a());
        assert_eq!(0, reference_pixels.get_color((36, 12)).a());
        for (source_point, output_point) in [((2, 2), (12, 12)), ((6, 2), (28, 12))] {
            let source_alpha = source_pixels.get_color(source_point).a();
            let svg_alpha = (f64::from(source_alpha) * opacity).round() as i16;
            let reference_alpha = i16::from(reference_pixels.get_color(output_point).a());
            assert!(
                (svg_alpha - reference_alpha).abs() <= 1,
                "draw opacity {draw_opacity}, source alpha {source_alpha}: SVG {svg_alpha}, Skia {reference_alpha}"
            );
        }
        assert_eq!(native_alpha, reference_pixels.get_color((12, 12)).a());
    }
}

#[test]
fn platform_surface_publishes_vectors_and_policy_revisions_without_a_window() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SvgSurface>();
    let surface = SvgSurface::new(SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        ..Default::default()
    })
    .unwrap();
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![Arc::new(surface.clone())];
    let interface = SvgPlatformRenderInterface::default();
    let backend = interface.create_backend_context(None);
    assert!(backend.is_ready_to_create_render_target(&surfaces));
    let target = backend.create_render_target(&surfaces);
    assert!(target.prefers_full_frame_rendering());
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(100, 60),
        2.0,
        CompositionTransparencyLevel::None,
    );
    let draw = || {
        let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
        context.set_transform(Matrix::create_scale(2.0, 2.0));
        let geometry = interface.create_rectangle_geometry(Rect::new(2.0, 3.0, 44.0, 24.0));
        context.draw_geometry(
            Some(&ImmutableSolidColorBrush::new(Colors::RED)),
            None,
            &*geometry,
        );
        context.dispose();
    };
    draw();
    let first = surface.take_frame().unwrap();
    assert_eq!(first.sequence, 1);
    assert_eq!(first.options_revision, 0);
    let output = first.result.unwrap();
    assert_eq!(output.report.vector_layers, 1);
    assert_eq!(output.report.embedded_images, 0);
    assert!(surface.take_frame().is_none());
    let revision = surface
        .set_options(SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            raster_scale: 1.0,
            ..Default::default()
        })
        .unwrap();
    draw();
    let second = surface.take_frame().unwrap();
    assert_eq!(second.sequence, 2);
    assert_eq!(second.options_revision, revision);
    assert_eq!(second.result.unwrap().report.raster_layers, 1);
    backend.dispose();
    assert!(backend.is_lost());
    assert!(!backend.is_ready_to_create_render_target(&surfaces));
}

#[test]
fn a_surface_failure_replaces_the_previous_result_instead_of_publishing_stale_artwork() {
    let surface = SvgSurface::new(SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        ..Default::default()
    })
    .unwrap();
    let backend = SvgPlatformRenderInterface::default().create_backend_context(None);
    let target = backend.create_render_target(&[Arc::new(surface.clone())]);
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(20, 20),
        1.0,
        CompositionTransparencyLevel::None,
    );
    {
        let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
        context.draw_ellipse(
            Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
            None,
            Rect::new(0.0, 0.0, 10.0, 10.0),
        );
        context.dispose();
    }
    {
        let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
        context.push_opacity(0.5, None);
        context.draw_ellipse(
            Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
            None,
            Rect::new(0.0, 0.0, 10.0, 10.0),
        );
        context.pop_opacity();
        context.dispose();
    }
    let frame = surface.take_frame().unwrap();
    assert_eq!(frame.sequence, 2);
    assert!(matches!(frame.result, Err(SvgError::VectorFidelity { .. })));
}

#[test]
fn unwinding_a_frame_publishes_a_recording_failure_and_allows_the_next_frame() {
    let surface = SvgSurface::default();
    let backend = SvgPlatformRenderInterface::default().create_backend_context(None);
    let target = backend.create_render_target(&[Arc::new(surface.clone())]);
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(20, 20),
        1.0,
        CompositionTransparencyLevel::None,
    );
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
        context.draw_ellipse(
            Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
            None,
            Rect::new(0.0, 0.0, 10.0, 10.0),
        );
        panic!("the frame producer failed after a balanced drawing command");
    }));
    assert!(panic.is_err());
    let failed = surface
        .take_frame()
        .expect("the aborted frame replaces the result");
    assert!(matches!(failed.result, Err(SvgError::Recording { .. })));
    let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
    context.dispose();
    let recovered = surface
        .take_frame()
        .expect("the target remains usable after unwind");
    assert!(recovered.sequence > failed.sequence);
    assert!(recovered.result.is_ok());
}

#[test]
fn producer_panic_with_an_active_native_lease_publishes_failure_without_double_unwind() {
    use ferroui_skia::{ISkiaApiLease, ISkiaApiLeaseFeature};
    use std::any::TypeId;
    use std::cell::RefCell;

    let surface = SvgSurface::default();
    let backend = SvgPlatformRenderInterface::default().create_backend_context(None);
    let target = backend.create_render_target(&[Arc::new(surface.clone())]);
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(20, 20),
        1.0,
        CompositionTransparencyLevel::None,
    );
    drop(target.try_create_drawing_context(&scene).unwrap());
    // Keep the previous success in the latest-result slot to detect stale
    // artwork if failed native finalization cannot publish a replacement.
    let held_lease = RefCell::<Option<Rc<dyn ISkiaApiLease>>>::new(None);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
        context.draw_ellipse(
            Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
            None,
            Rect::new(0.0, 0.0, 10.0, 10.0),
        );
        let feature = context
            .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
            .unwrap();
        *held_lease.borrow_mut() = Some(
            feature
                .downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>()
                .unwrap()
                .lease(),
        );
        panic!("original producer panic with a live native lease");
    }))
    .unwrap_err();
    assert_eq!(
        Some(&"original producer panic with a live native lease"),
        panic.downcast_ref::<&str>()
    );
    let failed = surface
        .take_frame()
        .expect("failed finalization replaces the old result");
    assert_eq!(2, failed.sequence);
    let Err(SvgError::Recording { reasons, .. }) = failed.result else {
        panic!("the incomplete picture must never be exported successfully");
    };
    assert!(reasons.iter().any(|reason| reason.contains("aborted")));
    assert!(reasons
        .iter()
        .any(|reason| reason.contains("could not be finalized")));
    held_lease.borrow_mut().take().unwrap().dispose();
    drop(target.try_create_drawing_context(&scene).unwrap());
    let recovered = surface.take_frame().unwrap();
    assert_eq!(3, recovered.sequence);
    assert!(recovered.result.is_ok());
}

#[test]
fn caches_reuse_recordings_and_respect_stricter_limits_on_every_export() {
    let mut document = SvgDocument::new(Size::new(20.0, 20.0)).unwrap();
    document
        .record_layer(SvgLayerOptions::new("shape", "Shape"), |context| {
            context.draw_rectangle(
                Some(&ImmutableSolidColorBrush::new(Colors::RED)),
                None,
                RoundedRect::from(Rect::new(1.0, 1.0, 18.0, 18.0)),
                &BoxShadows::default(),
            );
        })
        .unwrap();
    let options = SvgExportOptions::default();
    assert_eq!(
        document.export(&options).unwrap().report.vector_cache_hits,
        0
    );
    assert_eq!(
        document.export(&options).unwrap().report.vector_cache_hits,
        1
    );
    let raster = SvgExportOptions {
        fidelity: SvgFidelity::Raster,
        ..Default::default()
    };
    assert_eq!(
        document.export(&raster).unwrap().report.raster_cache_hits,
        0
    );
    assert_eq!(
        document.export(&raster).unwrap().report.raster_cache_hits,
        1
    );
    assert!(document
        .export(&SvgExportOptions {
            max_raster_pixels: 1,
            ..raster
        })
        .is_err());
    document.clear_export_cache();
    assert_eq!(
        document.export(&options).unwrap().report.vector_cache_hits,
        0
    );
}

#[test]
fn laid_out_controls_use_the_existing_immediate_rendering_path() {
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::Border;
    let _application = UnitTestApplication::start(
        TestServices::mock_platform_render_interface()
            .with_render_interface(Rc::new(SvgPlatformRenderInterface::default())),
    );
    let border = Border::new();
    border.set_width(60.0);
    border.set_height(30.0);
    border.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Colors::RED))));
    border.measure(Size::new(60.0, 30.0));
    border.arrange(Rect::new(0.0, 0.0, 60.0, 30.0));
    let mut document = SvgDocument::new(Size::new(60.0, 30.0)).unwrap();
    document
        .record_visual(SvgLayerOptions::new("control", "Control"), &border.upcast())
        .unwrap();
    let output = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        })
        .unwrap();
    assert!(output.report.draw_calls > 0);
    assert_eq!(output.report.vector_layers, 1);
    assert_eq!(output.report.raster_layers, 0);
}

#[test]
fn harfbuzz_shaped_ligatures_combining_marks_and_polish_text_export_as_paths() {
    use ferroui_base::media::{FlowDirection, FontFamily, FontManager, FormattedText, Typeface};
    use ferroui_base::platform::{IAssetLoader, StandardAssetLoader};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::utilities::locale;
    use ferroui_base::FerroLocator;
    struct Scope(Rc<dyn IDisposable>);
    impl Drop for Scope {
        fn drop(&mut self) {
            self.0.dispose();
        }
    }
    let _scope = Scope(FerroLocator::enter_scope());
    SvgPlatform::initialize();
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();
    let loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(loader);
    FontManager::current()
        .add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));
    let text = FormattedText::new(
        "office ffi — Zażółć gęślą jaźń — e\u{301}",
        locale!("pl-PL"),
        FlowDirection::LeftToRight,
        Typeface::new(FontFamily::parse("fonts:Inter#Inter").unwrap()),
        24.0,
        Some(Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK))),
    );
    let mut document = SvgDocument::new(Size::new(680.0, 80.0)).unwrap();
    document
        .record_layer(
            SvgLayerOptions::new("text", "Shaped text"),
            |implementation| {
                let mut core = PlatformDrawingContext::borrowed(implementation);
                let mut context = DrawingContext::new(&mut core);
                context.draw_text(&text, Point::new(12.0, 12.0));
                context.dispose();
            },
        )
        .unwrap();
    let output = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    assert!(xml.descendants().any(|n| n.has_tag_name("path")));
    assert!(!xml
        .descendants()
        .any(|n| n.has_tag_name("text") || n.has_tag_name("image")));
    let editable = document
        .export(&SvgExportOptions {
            text: SvgTextMode::Editable,
            ..Default::default()
        })
        .unwrap();
    assert!(editable
        .report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Unicode")));
}

#[test]
fn real_compositor_captures_complete_hidpi_frames_and_isolated_opacity() {
    use base64::Engine;
    use ferroui_base::media::{Brushes, Color};
    use ferroui_controls::testing::{CompositorTestServices, MockWindowingPlatform, TestServices};
    use ferroui_controls::{Border, Canvas, Control, Window};

    // Exercise the public window -> layout -> renderer -> compositor -> SVG
    // surface path. Only the OS window is mocked; drawing uses native Skia.
    let services = CompositorTestServices::start(
        TestServices::styled_window()
            .with_render_interface(Rc::new(SvgPlatformRenderInterface::default())),
    );
    let surface = SvgSurface::new(SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        ..Default::default()
    })
    .unwrap();
    let window_impl = MockWindowingPlatform::create_window_mock_with_size(64.0, 48.0);
    window_impl.render_scaling.set(2.0);
    window_impl.setup_surfaces(vec![Arc::new(surface.clone())]);
    services.setup(&window_impl);
    let window = Window::with_impl(window_impl);
    // Ensure cleanup even if an assertion fails, before the application scope
    // restores the dispatcher and platform services.
    struct CloseWindow(ferroui_base::Ref<Window>);
    impl Drop for CloseWindow {
        fn drop(&mut self) {
            self.0.close();
        }
    }
    let _close = CloseWindow(window.clone());
    window.set_background(Some(Brushes::white()));
    window.set_width(64.0);
    window.set_height(48.0);
    let canvas = Canvas::new();
    let left = Border::new();
    left.set_width(20.0);
    left.set_height(20.0);
    left.set_background(Some(Brushes::red()));
    Canvas::set_left(&left, 4.0);
    Canvas::set_top(&left, 4.0);
    let right = Border::new();
    right.set_width(20.0);
    right.set_height(20.0);
    right.set_background(Some(Brushes::blue()));
    Canvas::set_left(&right, 36.0);
    Canvas::set_top(&right, 4.0);
    canvas.children().add(left.clone().upcast::<Control>());
    canvas.children().add(right.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(&canvas)));
    window.show();
    services.run_jobs();

    let first = surface
        .take_frame()
        .expect("the compositor published its first frame");
    let output = first.result.unwrap();
    assert_eq!(1, output.report.vector_layers);
    assert_eq!(0, output.report.raster_layers);
    let has_fill = |output: &SvgOutput, color: Color| {
        let xml = roxmltree::Document::parse(&output.svg).unwrap();
        xml.descendants().any(|node| {
            node.attribute("fill")
                .and_then(|fill| Color::parse(fill).ok())
                == Some(color)
        })
    };
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    assert_eq!(Some("0 0 64 48"), xml.root_element().attribute("viewBox"));
    assert!(has_fill(&output, Colors::RED), "{}", output.svg);
    assert!(has_fill(&output, Colors::BLUE), "{}", output.svg);

    // Invalidate one sibling only. A capture target must redraw the unchanged
    // sibling too; a normal incremental target would preserve its old pixels.
    left.set_background(Some(Brushes::green()));
    services.run_jobs();
    let second = surface
        .take_frame()
        .expect("the changed control produced a new frame");
    assert!(second.sequence > first.sequence);
    let output = second.result.unwrap();
    assert_eq!(1, output.report.vector_layers);
    assert!(has_fill(&output, Colors::GREEN), "{}", output.svg);
    assert!(has_fill(&output, Colors::BLUE), "{}", output.svg);
    assert!(!has_fill(&output, Colors::RED), "{}", output.svg);

    // This state change is applied by the actual compositor, not by the
    // test calling push_opacity. Its complete frame must be rasterized once.
    surface
        .set_options(SvgExportOptions {
            raster_scale: 1.0,
            ..Default::default()
        })
        .unwrap();
    left.set_opacity(0.5);
    services.run_jobs();
    let third = surface
        .take_frame()
        .expect("the opacity change produced a new frame");
    assert!(third.sequence > second.sequence);
    let output = third.result.unwrap();
    assert_eq!(1, output.report.raster_layers, "{:?}", output.report);
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let href = xml
        .descendants()
        .find(|node| node.has_tag_name("image"))
        .unwrap()
        .attributes()
        .find(|attribute| attribute.name() == "href")
        .unwrap()
        .value();
    let png = base64::engine::general_purpose::STANDARD
        .decode(href.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&png))
        .unwrap()
        .make_raster_image(None, skia_safe::image::CachingHint::Allow)
        .unwrap();
    let pixels = image.peek_pixels().unwrap();
    // The recorded canvas is in device pixels (2x), then mapped to logical
    // SVG units. Sampling away from edges also catches doubled DPI scaling.
    assert_eq!((128, 96), (pixels.width(), pixels.height()));
    assert_eq!(skia_safe::Color::BLUE, pixels.get_color((80, 20)));
    assert_eq!(skia_safe::Color::WHITE, pixels.get_color((60, 20)));
    let green = pixels.get_color((20, 20));
    assert_eq!(255, green.a());
    assert!((127..=128).contains(&green.r()), "{green:?}");
    assert!((191..=192).contains(&green.g()), "{green:?}");
    assert!((127..=128).contains(&green.b()), "{green:?}");
}
