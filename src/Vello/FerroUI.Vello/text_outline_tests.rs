//! Regression coverage for explicitly unhinted, scalable glyph drawing.
use super::*;
use crate::scene::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneGlyphRun, VelloScenePaint,
};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableImageBrush, ImmutableLinearGradientBrush, ImmutableTransform,
};
use ferroui_base::media::{AlignmentX, AlignmentY, GradientSpreadMethod, Stretch, TileMode};
use ferroui_base::platform::AlphaFormat;
use ferroui_base::{RelativePoint, RelativeUnit};
use kurbo::{Affine, BezPath, Shape, Stroke};
use peniko::{BlendMode, Fill};
use skrifa::MetadataProvider;

fn scalable_options() -> TextOptions {
    TextOptions {
        text_hinting_mode: TextHintingMode::None,
        baseline_pixel_alignment: BaselinePixelAlignment::Unaligned,
        text_rendering_mode: TextRenderingMode::Antialias,
    }
}

fn max_pixel_difference(first: &TextTarget, second: &TextTarget) -> u8 {
    let first = first.pixels();
    let second = second.pixels();
    first
        .iter()
        .flatten()
        .zip(second.iter().flatten())
        .map(|(a, b)| {
            a.0.abs_diff(b.0)
                .max(a.1.abs_diff(b.1))
                .max(a.2.abs_diff(b.2))
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn synthetic_bold_public_outlines_keep_the_canonical_shape_at_every_size() {
    let plain = VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::None).unwrap();
    let h = plain.face().font_ref().charmap().map('H').unwrap().to_u32() as u16;
    for simulations in [FontSimulations::Bold, FontSimulations::Bold | FontSimulations::Oblique] {
        let face = VelloTypeface::from_bytes(INTER.to_vec(), simulations).unwrap();
        for character in ['H', 'o', '&', '@', ' '] {
            let glyph = face.face().font_ref().charmap().map(character).unwrap().to_u32() as u16;
            let canonical = face.face().glyph_path(glyph, 64.0).unwrap();
            for size in [9.0, 12.0, 20.0, 28.0, 36.0, 72.0] {
                let outline = face.face().glyph_path(glyph, size).unwrap();
                assert_eq!(
                    outline,
                    Affine::scale(size / 64.0) * &canonical,
                    "{character:?} at {size} with {simulations:?} must scale the completed public path"
                );
            }
        }
        if simulations == FontSimulations::Bold {
            for size in [9.0, 20.0, 28.0, 72.0] {
                let before = kurbo::Shape::bounding_box(&plain.face().glyph_path(h, size).unwrap());
                let after = kurbo::Shape::bounding_box(&face.face().glyph_path(h, size).unwrap());
                // Skia's public paths apply fake bold at size 64, hence a
                // stroke width of 1/32 em at every requested size. Size 28
                // must grow by 0.875, not the native glyph's ~0.9614 width.
                let width = size / 32.0;
                assert!((after.width() - before.width() - width).abs() < 1e-9);
                assert!((after.height() - before.height() - width).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn unhinted_curved_glyph_pixels_match_public_outlines_at_fractional_affine_views() {
    let scope = start();
    let foreground = ImmutableSolidColorBrush::new(Colors::BLACK);
    let views = [
        Matrix::IDENTITY,
        Matrix::new(0.8, 0.0, 0.0, 1.1, 10.375, 7.625),
        Matrix::new(0.95, 0.08, 0.12, 0.9, 2.25, 1.75),
    ];
    let faces = [
        VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::None).unwrap(),
        VelloTypeface::from_bytes(
            INTER.to_vec(),
            FontSimulations::Bold | FontSimulations::Oblique,
        )
        .unwrap(),
        VelloTypeface::from_bytes_with_variations(
            INTER_VARIABLE.to_vec(),
            FontSimulations::None,
            &[("wght", 625.0)],
        )
        .unwrap(),
    ];
    for face in faces {
        let run = glyph_run_of(face, "o", 27.25, Point::new(23.125, 61.375));
        let mut infos = run.glyph_infos().borrow().to_vec();
        infos[0].glyph_offset = Vector::new(1.23456789, -0.87654321);
        run.set_glyph_infos(GlyphInfoList::from(infos));
        let geometry = run.try_build_geometry().unwrap().platform_impl().unwrap();
        for view in views {
            let drawn = TextTarget::new(160, 120);
            drawn.draw(|context| {
                context.set_transform(view);
                context.push_text_options(scalable_options());
                context.draw_glyph_run(Some(&foreground), &*run.platform_impl());
                context.pop_text_options();
            });
            let outlined = TextTarget::new(160, 120);
            outlined.draw(|context| {
                context.set_transform(view);
                context.draw_geometry(Some(&foreground), None, &*geometry);
            });
            assert!(
                drawn.ink(is_not_white).1 > 50,
                "the curved glyph must leave measurable ink"
            );
            let delta = max_pixel_difference(&drawn, &outlined);
            assert!(
                delta <= 1,
                "unhinted curves and exported geometry differ by {delta}/255 at {view:?}"
            );
        }
    }
    scope.dispose();
}

#[test]
fn unhinted_outline_drawing_keeps_translucent_glyphs_as_separate_source_over_passes() {
    let scope = start();
    let face = VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::None).unwrap();
    let origin = Point::new(24.25, 68.375);
    let run = glyph_run_of(face.clone(), "oo", 40.25, origin);
    let mut infos = run.glyph_infos().borrow().to_vec();
    assert_eq!(infos.len(), 2);
    infos[0].glyph_advance = 0.0;
    run.set_glyph_infos(GlyphInfoList::from(infos));
    let single = glyph_run_of(face, "o", 40.25, origin)
        .try_build_geometry()
        .unwrap()
        .platform_impl()
        .unwrap();
    let foreground = ImmutableSolidColorBrush::with_opacity(Colors::BLACK, 0.5);
    let drawn = TextTarget::new(120, 100);
    drawn.draw(|context| {
        context.push_text_options(scalable_options());
        context.draw_glyph_run(Some(&foreground), &*run.platform_impl());
        context.pop_text_options();
    });
    let twice = TextTarget::new(120, 100);
    twice.draw(|context| {
        context.draw_geometry(Some(&foreground), None, &*single);
        context.draw_geometry(Some(&foreground), None, &*single);
    });
    let once = TextTarget::new(120, 100);
    once.draw(|context| context.draw_geometry(Some(&foreground), None, &*single));
    assert!(max_pixel_difference(&drawn, &twice) <= 1);
    assert!(
        max_pixel_difference(&drawn, &once) >= 60,
        "coincident glyphs must accumulate paint alpha"
    );
    scope.dispose();
}

#[derive(Default)]
struct Recorded {
    fills: Vec<(BezPath, Affine, VelloScenePaint, BlendMode, bool)>,
    native_runs: Vec<(Affine, VelloScenePaint, usize, bool)>,
    clips: Vec<(BezPath, Affine)>,
    clip_pops: usize,
}

/// Observe the backend-neutral scene boundary without invoking a GPU or a
/// glyph rasterizer. This checks that fallback artwork is still delegated and
/// that run-relative paints do not move with the new per-glyph transforms.
struct RecordingSink(Rc<RefCell<Recorded>>);

impl IVelloSceneSink for RecordingSink {
    fn rendering_mode(&self) -> VelloRenderingMode {
        VelloRenderingMode::Cpu
    }
    fn capabilities(&self) -> VelloSceneCapabilities {
        VelloSceneCapabilities {
            blend_layers: true,
            aliased_edges: true,
            aliased_rectangles: true,
            image_paints: true,
            read_back: false,
            device_textures: false,
            retained_targets: false,
        }
    }
    fn width(&self) -> u16 {
        200
    }
    fn height(&self) -> u16 {
        140
    }
    fn reset(&mut self) {
        *self.0.borrow_mut() = Recorded::default();
    }
    fn fill(
        &mut self,
        path: &BezPath,
        _rule: Fill,
        transform: Affine,
        paint: &VelloScenePaint,
        blend: BlendMode,
        aa: bool,
    ) {
        self.0
            .borrow_mut()
            .fills
            .push((path.clone(), transform, paint.clone(), blend, aa));
    }
    fn stroke(&mut self, _: &BezPath, _: &Stroke, _: Affine, _: &VelloScenePaint, _: bool) {
        panic!("a glyph run does not issue strokes");
    }
    fn draw_glyph_run(
        &mut self,
        run: &VelloSceneGlyphRun<'_>,
        transform: Affine,
        paint: &VelloScenePaint,
        _: bool,
    ) {
        self.0.borrow_mut().native_runs.push((
            transform,
            paint.clone(),
            run.glyphs.len(),
            run.hint,
        ));
    }
    fn push_clip(&mut self, path: &BezPath, _: Fill, transform: Affine, _: bool) {
        self.0.borrow_mut().clips.push((path.clone(), transform));
    }
    fn pop_clip(&mut self) {
        self.0.borrow_mut().clip_pops += 1;
    }
    fn push_layer(&mut self, _: BlendMode, _: f32) {}
    fn pop_layer(&mut self) {}
    fn render_to_pixels(&mut self, _: &mut [u8]) {
        panic!("recording only");
    }
}

fn record(run: &GlyphRun, brush: &dyn IBrush, options: TextOptions, view: Matrix) -> Recorded {
    let recorded = Rc::new(RefCell::new(Recorded::default()));
    let mut context = DrawingContextImpl::new(
        CreateInfo {
            sink: Box::new(RecordingSink(recorded.clone())),
            backdrop: None,
            on_finished: Box::new(|_| {}),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![VelloRenderingMode::Cpu],
        },
        Vec::new(),
    );
    context.set_transform(view);
    context.push_text_options(options);
    context.draw_glyph_run(Some(brush), &*run.platform_impl());
    context.pop_text_options();
    context.dispose();
    let result = recorded.take();
    result
}

fn assert_affine_close(actual: Affine, expected: Affine) {
    for (a, b) in actual.as_coeffs().into_iter().zip(expected.as_coeffs()) {
        assert!((a - b).abs() < 1e-9, "{actual:?} != {expected:?}");
    }
}

#[test]
fn unhinted_outline_drawing_preserves_run_relative_gradient_and_image_coordinates() {
    let scope = start();
    let face = VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::None).unwrap();
    let run = glyph_run_of(face, "o o", 29.25, Point::new(17.375, 65.625));
    let gradient = ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0.0, Colors::RED),
            ImmutableGradientStop::new(1.0, Colors::BLUE),
        ],
        1.0,
        Some(Rc::new(ImmutableTransform::new(
            Matrix::create_translation(2.75, -1.125),
        ))),
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.6, RelativeUnit::Relative)),
        None,
    );
    let interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .unwrap();
    let bitmap = Rc::new(Bitmap::from_impl(interface.load_bitmap_from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &[
            255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255,
        ],
        PixelSize::new(2, 2),
        Vector::new(96.0, 96.0),
        8,
    )));
    let image = ImmutableImageBrush::new(
        Some(bitmap),
        AlignmentX::Center,
        AlignmentY::Center,
        None,
        1.0,
        Some(Rc::new(ImmutableTransform::new(
            Matrix::create_translation(-1.25, 3.375),
        ))),
        RelativePoint::default(),
        None,
        Stretch::Fill,
        TileMode::None,
        None,
    );
    for brush in [&gradient as &dyn IBrush, &image as &dyn IBrush] {
        for view in [
            Matrix::new(0.8, 0.0, 0.0, 1.1, 10.375, 7.625),
            Matrix::new(0.95, 0.08, 0.12, 0.9, 2.25, 1.75),
        ] {
            for options in [
                scalable_options(),
                TextOptions {
                    baseline_pixel_alignment: BaselinePixelAlignment::Aligned,
                    ..scalable_options()
                },
                TextOptions {
                    text_rendering_mode: TextRenderingMode::Alias,
                    baseline_pixel_alignment: BaselinePixelAlignment::Aligned,
                    ..scalable_options()
                },
            ] {
                let native = record(
                    &run,
                    brush,
                    TextOptions {
                        text_hinting_mode: TextHintingMode::Strong,
                        ..options
                    },
                    view,
                );
                let outlined = record(&run, brush, options, view);
                assert_eq!(native.native_runs.len(), 1);
                assert!(native.fills.is_empty());
                assert_eq!(outlined.fills.len(), 2, "the space must not create a draw");
                assert!(outlined.native_runs.is_empty());
                let (native_transform, native_paint, _, _) = &native.native_runs[0];
                for (_, transform, paint, blend, aa) in &outlined.fills {
                    assert_affine_close(
                        *transform * paint.transform,
                        *native_transform * native_paint.transform,
                    );
                    assert_eq!(*blend, BlendMode::default());
                    assert_eq!(*aa, options.text_rendering_mode != TextRenderingMode::Alias);
                    match (&paint.brush, &native_paint.brush) {
                        (VelloSceneBrush::Gradient(a), VelloSceneBrush::Gradient(b)) => {
                            assert_eq!(a, b)
                        }
                        (VelloSceneBrush::Image(a), VelloSceneBrush::Image(b)) => {
                            // Each brush resolution renders its own tile. Its
                            // resource identity differs, but its pixels and
                            // placement must remain identical.
                            assert_eq!(a.image.data.data(), b.image.data.data());
                            assert_eq!(
                                (
                                    a.image.width,
                                    a.image.height,
                                    a.image.format,
                                    a.image.alpha_type
                                ),
                                (
                                    b.image.width,
                                    b.image.height,
                                    b.image.format,
                                    b.image.alpha_type
                                )
                            );
                            assert_eq!(
                                (a.x_extend, a.y_extend, a.quality, a.alpha),
                                (b.x_extend, b.y_extend, b.quality, b.alpha)
                            );
                        }
                        _ => panic!("the original brush must reach every glyph unchanged"),
                    }
                }
                assert_eq!(
                    outlined.clips, native.clips,
                    "tile clips remain in run coordinates"
                );
                assert_eq!(outlined.clip_pops, native.clip_pops);
            }
        }
    }
    scope.dispose();
}

#[test]
fn unhinted_outline_drawing_retains_native_hinted_colour_bitmap_and_partial_runs() {
    let scope = start();
    let foreground = ImmutableSolidColorBrush::new(Colors::BLACK);
    let face = VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::None).unwrap();
    let run = glyph_run_of(face, "oo", 30.0, Point::new(4.25, 42.375));
    for hint in [
        TextHintingMode::Unspecified,
        TextHintingMode::Light,
        TextHintingMode::Strong,
    ] {
        let recorded = record(
            &run,
            &foreground,
            TextOptions {
                text_hinting_mode: hint,
                ..scalable_options()
            },
            Matrix::IDENTITY,
        );
        assert!(recorded.fills.is_empty());
        assert_eq!(recorded.native_runs.len(), 1);
        assert!(recorded.native_runs[0].3);
    }
    let mut infos = run.glyph_infos().borrow().to_vec();
    infos[1].glyph_index = u16::MAX;
    run.set_glyph_infos(GlyphInfoList::from(infos));
    assert!(run.try_build_geometry().is_none());
    let recorded = record(&run, &foreground, scalable_options(), Matrix::IDENTITY);
    assert!(
        recorded.fills.is_empty(),
        "a partial run must not silently lose its missing glyph"
    );
    assert_eq!(recorded.native_runs.len(), 1);
    assert!(!recorded.native_runs[0].3);

    for (font, text) in [
        (
            &include_bytes!(
                "../../Skia/FerroUI.Skia/test_assets/assets/TwitterColorEmoji-SVGinOT.ttf"
            )[..],
            "😀",
        ),
        (
            &include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/NISC18030.ttf")[..],
            "中",
        ),
    ] {
        let face = VelloTypeface::from_bytes(font.to_vec(), FontSimulations::None).unwrap();
        let drawable = face.face().is_drawable();
        let run = glyph_run_of(face, text, 30.0, Point::new(4.25, 42.375));
        assert!(run.try_build_geometry().is_none());
        let recorded = record(&run, &foreground, scalable_options(), Matrix::IDENTITY);
        assert!(
            recorded.fills.is_empty(),
            "colour and bitmap artwork must retain its native pipeline"
        );
        assert_eq!(recorded.native_runs.len(), usize::from(drawable));
        if drawable {
            assert!(!recorded.native_runs[0].3);
        }
    }
    scope.dispose();
}

#[test]
fn unhinted_outline_cache_keeps_sub_float_shaping_offsets_and_invalidates_with_the_run() {
    let scope = start();
    let face = VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::None).unwrap();
    let run = glyph_run_of(face, "o", 29.25, Point::new(11.125, 61.375));
    let first = run.try_build_geometry().unwrap().bounds();
    // At x=32, an f32 position cannot retain this displacement. The scalable
    // contract must retain it even though it is smaller than a screen pixel.
    let offset = Vector::new(32.000000123, -0.123456789);
    let mut infos = run.glyph_infos().borrow().to_vec();
    infos[0].glyph_offset = offset;
    run.set_glyph_infos(GlyphInfoList::from(infos));
    let displaced = run.try_build_geometry().unwrap().bounds();
    assert!((displaced.x - first.x - offset.x).abs() < 1e-11);
    assert!((displaced.y - first.y - offset.y).abs() < 1e-11);
    let foreground = ImmutableSolidColorBrush::new(Colors::BLACK);
    let recorded = record(&run, &foreground, scalable_options(), Matrix::IDENTITY);
    let (path, transform, _, _, _) = &recorded.fills[0];
    let bounds = kurbo::Shape::bounding_box(&(*transform * path));
    assert!((bounds.x0 - displaced.x).abs() < 1e-11);
    assert!((bounds.y0 - displaced.y).abs() < 1e-11);
    run.set_baseline_origin(Point::new(21.375, 67.625));
    let moved = run.try_build_geometry().unwrap().bounds();
    assert!((moved.x - displaced.x - 10.25).abs() < 1e-11);
    assert!((moved.y - displaced.y - 6.25).abs() < 1e-11);
    scope.dispose();
}

#[test]
fn unhinted_preparation_cache_preserves_pixels_clip_paints_and_separate_glyph_composition() {
    let scope = start();
    let face = VelloTypeface::from_bytes(INTER.to_vec(), FontSimulations::Bold | FontSimulations::Oblique).unwrap();
    let run = glyph_run_of(face, "oo", 34.25, Point::new(31.125, 80.375));
    let mut infos = run.glyph_infos().borrow().to_vec();
    infos[0].glyph_advance = 8.000000123;
    infos[1].glyph_offset = Vector::new(0.123456789, -0.234567891);
    run.set_glyph_infos(GlyphInfoList::from(infos));
    let platform = run.platform_impl();
    let native = platform.as_any().downcast_ref::<GlyphRunImpl>().unwrap();
    let solid = ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.47);
    let gradient = ImmutableLinearGradientBrush::new(
        &[ImmutableGradientStop::new(0.0, Colors::RED), ImmutableGradientStop::new(1.0, Colors::BLUE)],
        0.63, Some(Rc::new(ImmutableTransform::new(Matrix::create_translation(2.75, -1.125)))),
        None, GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.6, RelativeUnit::Relative)), None,
    );
    let interface = FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().unwrap();
    let bitmap = Rc::new(Bitmap::from_impl(interface.load_bitmap_from_pixels(
        PixelFormat::RGBA8888, AlphaFormat::Premul,
        &[255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255],
        PixelSize::new(2, 2), Vector::new(96.0, 96.0), 8,
    )));
    let image = ImmutableImageBrush::new(Some(bitmap), AlignmentX::Center, AlignmentY::Center,
        None, 0.71, Some(Rc::new(ImmutableTransform::new(Matrix::create_translation(-1.25, 3.375)))),
        RelativePoint::default(), None, Stretch::Fill, TileMode::None, None);
    let clip = kurbo::Rect::new(35.375, 34.125, 91.625, 102.75).to_path(0.05);
    let mut last_misses = None;
    for view in [
        Matrix::new(0.95, 0.08, 0.12, 0.9, 2.25, 1.75),
        Matrix::new(0.95, 0.08, 0.12, 0.9, 7.625, -2.125),
        Matrix::new(1.125, 0.08, 0.12, 0.9, 7.625, -2.125),
    ] {
        for brush in [&solid as &dyn IBrush, &gradient as &dyn IBrush, &image as &dyn IBrush] {
            let recorded = record(&run, brush, scalable_options(), view);
            assert_eq!(recorded.fills.len(), 2);
            assert!(recorded.fills.iter().any(|(path, ..)| path.elements().iter()
                .any(|item| matches!(item, kurbo::PathEl::QuadTo(..) | kurbo::PathEl::CurveTo(..)))),
                "Default recording must retain the canonical curves");
            let render = |cached: bool| {
                let mut sink = crate::scene::VelloCpuSceneSink::new(160, 120);
                sink.push_clip(&clip, Fill::NonZero, Affine::IDENTITY, true);
                for (index, (path, transform, paint, blend, aa)) in recorded.fills.iter().enumerate() {
                    if cached {
                        sink.fill_glyph_outline(path, *transform, paint, *aa,
                            &|transform| native.prepared_outline(index, transform));
                    } else {
                        sink.fill(path, Fill::NonZero, *transform, paint, *blend, *aa);
                    }
                }
                sink.pop_clip();
                let mut pixels = vec![0; 160 * 120 * 4];
                sink.render_to_pixels(&mut pixels);
                pixels
            };
            let canonical = render(false);
            let cached = render(true);
            assert!(cached.iter().any(|value| *value != 0));
            let difference = cached.iter().zip(&canonical).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
            assert!(difference <= 1, "Cached glyph coverage/paint differs by {difference}/255 at {view:?}");
        }
        let (_, bytes, hits, misses) = native.outline_cache_stats();
        assert!(bytes > 0 && bytes <= 128 * 1024);
        assert!(hits >= 2, "Changing only paint must reuse glyph geometry");
        if view.m11 == 0.95 {
            if let Some(previous) = last_misses { assert_eq!(misses, previous, "Panning must not flatten again"); }
            last_misses = Some(misses);
        } else { assert!(misses > last_misses.unwrap(), "Changing scale must prepare a fresh contour"); }
    }
    run.set_font_rendering_em_size(41.375);
    let replacement = run.platform_impl();
    assert!(!std::sync::Arc::ptr_eq(&platform, &replacement));
    assert_eq!(replacement.as_any().downcast_ref::<GlyphRunImpl>().unwrap().outline_cache_stats(), (0, 0, 0, 0),
        "An edited text run cannot inherit another run's device geometry");
    scope.dispose();
}
