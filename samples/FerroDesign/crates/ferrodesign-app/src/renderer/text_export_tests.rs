use super::*;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::{FontManager, Pen};
use ferroui_base::platform::{IAssetLoader, PixelFormat, StandardAssetLoader};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{FerroLocator, PixelRect, PixelSize, Vector};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(None)));
    ferroui_vello::VelloPlatform::initialize();
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();
    FontManager::current().add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));
    scope
}

fn run() -> Rc<GlyphRun> {
    let face = typeface("fonts:Inter#Inter", 400, false).glyph_typeface();
    let glyph = face.character_to_glyph_map().get_glyph('o' as i32);
    let run = GlyphRun::from_glyph_indices(face, 36.25, ReadOnlyMemory::<u16>::from_str("oo"),
        &[glyph, glyph], Some(UiPoint::new(5.125, 48.375)), 0);
    let mut infos = run.glyph_infos().borrow().to_vec();
    infos[0].glyph_advance = 5.000000123;
    infos[1].glyph_offset = Vector::new(0.123456789, -0.234567891);
    run.set_glyph_infos(infos.into());
    run
}

fn pixels(paint: impl FnOnce(&mut DrawingContext<'_>)) -> Vec<u8> {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(128, 96));
    paint(&mut bitmap.create_drawing_context());
    let mut bytes = vec![0; 128 * 96 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 128, 96), &mut bytes, 128 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in bytes.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    bytes
}

fn geometry(path: &GeometryPath, filled: bool) -> Ref<Geometry> {
    let geometry = StreamGeometry::new();
    {
        let mut context = geometry.open();
        context.set_fill_rule(path.fill_rule);
        let mut open = false;
        for segment in &path.segments {
            match *segment {
                GeometryPathSegment::MoveTo(point) => {
                    if open { context.end_figure(false); }
                    context.begin_figure(point, filled);
                    open = true;
                }
                GeometryPathSegment::LineTo(point) => context.line_to(point, true),
                GeometryPathSegment::QuadraticTo { control, point } => context.quadratic_bezier_to(control, point, true),
                GeometryPathSegment::CubicTo { control1, control2, point } => context.cubic_bezier_to(control1, control2, point, true),
                GeometryPathSegment::Close => { context.end_figure(true); open = false; }
            }
        }
        if open { context.end_figure(false); }
    }
    geometry.upcast()
}

fn replay(artwork: &TextArtwork, context: &mut DrawingContext<'_>) {
    for operation in &artwork.operations {
        let fill = operation.fill.map(|paint| Rc::new(ImmutableSolidColorBrush::with_opacity(paint.color, paint.opacity)) as Rc<dyn IBrush>);
        let stroke: Option<Rc<dyn IPen>> = operation.stroke.as_ref().map(|stroke| {
            Pen::with_all(Some(Rc::new(ImmutableSolidColorBrush::with_opacity(stroke.paint.color, stroke.paint.opacity))),
                stroke.thickness, stroke.dashes.as_ref().map(|values| {
                    Rc::new(ImmutableDashStyle::new(Some(values), stroke.dash_offset)) as Rc<dyn ferroui_base::media::IDashStyle>
                }), stroke.cap, stroke.join, stroke.miter_limit).into()
        });
        let state = context.push_transform(operation.transform);
        context.draw_geometry(fill.as_ref(), stroke.as_ref(), &geometry(&operation.path, operation.fill.is_some()));
        context.pop(state);
    }
}

#[test]
fn text_export_keeps_overlapping_glyph_alpha_original_brush_box_and_sub_float_offsets() {
    let scope = backend();
    let run = run();
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_opacity(
        ferroui_base::media::Color::from_argb(191, 220, 30, 90), 0.57));
    for transform in [Matrix::IDENTITY, Matrix::new(1.15, 0.08, -0.13, 0.9, 21.125, 9.375)] {
        let mut recorder = Recorder::new(Limits::default());
        recorder.push_transform(transform);
        recorder.draw_glyph_run(Some(&brush), &run);
        recorder.pop_transform();
        let artwork = recorder.finish().unwrap();
        assert_eq!((artwork.glyph_runs, artwork.glyphs, artwork.operations.len()), (1, 2, 2));
        assert!(artwork.operations.iter().all(|operation| operation.brush_bounds == run.bounds()
            && operation.fill == Some(SolidPaint { color: brush.as_solid_color_brush().unwrap().color(), opacity: 0.57 })));
        let native = pixels(|context| {
            let text = context.push_text_options(typography::design_text_options());
            let state = context.push_transform(transform);
            context.draw_glyph_run(Some(&brush), &run);
            context.pop(state);
            context.pop(text);
        });
        let exported = pixels(|context| replay(&artwork, context));
        let max_error = native.iter().zip(&exported).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(max_error <= 1, "Per-glyph source-over outlines changed a channel by {max_error}/255");
        // A combined glyph-run fill cannot attain this overlap alpha. This
        // assertion fails if the writer merges the two glyphs into one draw.
        let single_alpha = (191.0_f64 * 0.57).ceil() as u8;
        assert!(exported.chunks_exact(4).any(|pixel| pixel[3] > single_alpha + 20));
    }
    scope.dispose();
}

#[test]
fn text_export_rejects_a_later_unavailable_glyph_and_budget_exhaustion_as_complete_results() {
    let scope = backend();
    let run = run();
    let brush = solid_brush(Color::BLACK, 1.0);
    let mut infos = run.glyph_infos().borrow().to_vec();
    infos[1].glyph_index = u16::MAX;
    run.set_glyph_infos(infos.into());
    let mut recorder = Recorder::new(Limits::default());
    recorder.draw_glyph_run(Some(&brush), &run);
    assert_eq!(recorder.artwork.operations.len(), 1, "the first valid glyph was visited");
    assert!(recorder.finish().unwrap_err().contains("unavailable glyph outline"));

    let run = super::tests::run();
    let mut recorder = Recorder::new(Limits { glyphs: 1, ..Limits::default() });
    recorder.draw_glyph_run(Some(&brush), &run);
    assert!(recorder.finish().unwrap_err().contains("glyph outline budget"));
    let mut recorder = Recorder::new(Limits { segments: 1, ..Limits::default() });
    recorder.draw_glyph_run(Some(&brush), &run);
    assert!(recorder.finish().unwrap_err().contains("curve budget"));
    scope.dispose();
}

#[test]
fn text_export_retains_decoration_dash_cap_join_opacity_and_nested_transform_order() {
    let scope = backend();
    let fill = solid_brush(Color::rgba(10, 160, 90, 170), 0.63);
    let pen: Rc<dyn IPen> = Pen::with_all(Some(fill.clone()), 2.375,
        Some(Rc::new(ImmutableDashStyle::new(Some(&[2.0, 1.25]), 0.375))), PenLineCap::Round, PenLineJoin::Bevel, 7.0).into();
    let outer = Matrix::new(1.0, 0.1, 0.2, 0.9, 8.125, 4.375);
    let inner = Matrix::create_translation(7.25, 11.5);
    let draw = |sink: &mut dyn ITextDrawingSink| {
        sink.push_transform(outer);
        sink.push_transform(inner);
        sink.draw_rectangle(Some(&fill), Some(&pen), UiRect::new(5.25, 2.75, 40.5, 20.125));
        sink.draw_line(&pen, UiPoint::new(2.5, 30.25), UiPoint::new(64.75, 31.875));
        sink.pop_transform();
        sink.pop_transform();
    };
    let mut recorder = Recorder::new(Limits::default());
    draw(&mut recorder);
    let artwork = recorder.finish().unwrap();
    assert_eq!(artwork.operations.len(), 2);
    for operation in &artwork.operations {
        assert_eq!(operation.transform, inner * outer);
        let stroke = operation.stroke.as_ref().unwrap();
        assert_eq!((stroke.thickness, stroke.cap, stroke.join, stroke.miter_limit, stroke.dash_offset),
            (2.375, PenLineCap::Round, PenLineJoin::Bevel, 7.0, 0.375));
        assert_eq!(stroke.dashes, Some(vec![2.0, 1.25]));
    }
    let native = pixels(|context| draw(context));
    let exported = pixels(|context| replay(&artwork, context));
    let max_error = native.iter().zip(&exported).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    assert!(max_error <= 1, "Native decoration recording changed a channel by {max_error}/255");
    scope.dispose();
}
