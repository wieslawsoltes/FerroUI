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
