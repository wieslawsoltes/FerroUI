use super::*;
use ferroui_base::media::effects::ImmutableBlurEffect;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::Colors;
use ferroui_base::{Size, Vector};
use ferroui_skia::{ISkiaApiLeaseFeature, PictureRenderTarget};

fn recording() -> (PictureRenderTarget, SvgDrawingContext) {
    let target = PictureRenderTarget::new(None, None, Vector::new(96.0, 96.0));
    let context = SvgDrawingContext::new(Box::new(
        target.create_export_drawing_context(Size::new(64.0, 64.0), false),
    ));
    (target, context)
}

fn fill(context: &mut dyn IDrawingContextImpl, color: Color, bounds: Rect) {
    context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(color)),
        None,
        RoundedRect::from_rect(bounds),
        &BoxShadows::default(),
    );
}

fn pixels(picture: &skia_safe::Picture) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((64, 64)).unwrap();
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    surface.canvas().draw_picture(picture, None, None);
    let info = skia_safe::ImageInfo::new(
        (64, 64),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let mut pixels = vec![0; 64 * 64 * 4];
    assert!(surface.read_pixels(&info, &mut pixels, 64 * 4, (0, 0)));
    pixels
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
        .try_into()
        .unwrap()
}

#[test]
fn ordinary_vectors_remain_vector_and_clip_state_is_restored() {
    let (target, mut context) = recording();
    context.push_clip(Rect::new(0.0, 0.0, 16.0, 16.0));
    context.set_transform(Matrix::create_translation(8.0, 0.0));
    fill(&mut context, Colors::RED, Rect::new(0.0, 0.0, 32.0, 32.0));
    context.pop_clip();
    assert_eq!(Matrix::IDENTITY, context.transform());
    context.finish();
    let audit = context.audit();
    assert!(audit.raster_reasons.is_empty(), "{audit:?}");
    assert!(audit.errors.is_empty(), "{audit:?}");
    assert_eq!(1, audit.draw_calls);
    let image = pixels(&target.get_picture());
    assert_eq!([255, 0, 0, 255], pixel(&image, 10, 8));
    assert_eq!([0, 0, 0, 0], pixel(&image, 20, 8));
}

#[test]
fn render_and_text_options_keep_their_independent_platform_stacks() {
    let (target, mut context) = recording();
    context.push_render_options(RenderOptions::default());
    context.push_clip(Rect::new(0.0, 0.0, 16.0, 16.0));
    context.push_text_options(TextOptions::default());
    context.pop_render_options();
    context.set_transform(Matrix::create_translation(8.0, 0.0));
    fill(&mut context, Colors::RED, Rect::new(0.0, 0.0, 32.0, 32.0));
    context.pop_clip();
    assert_eq!(Matrix::IDENTITY, context.transform());
    context.pop_text_options();
    fill(&mut context, Colors::BLUE, Rect::new(40.0, 0.0, 16.0, 16.0));
    context.finish();
    assert!(context.audit().errors.is_empty(), "{:?}", context.audit());
    assert!(context.audit().raster_reasons.is_empty());
    let image = pixels(&target.get_picture());
    assert_eq!([255, 0, 0, 255], pixel(&image, 12, 8));
    assert_eq!([0, 0, 0, 0], pixel(&image, 20, 8));
    assert_eq!([0, 0, 255, 255], pixel(&image, 44, 8));
}

#[test]
fn isolated_opacity_composes_the_overlap_once() {
    let (target, mut context) = recording();
    assert!(context.try_push_compositing_layer(
        BitmapBlendingMode::SourceOver,
        0.5,
        Rect::new(0.0, 0.0, 64.0, 64.0)
    ));
    fill(&mut context, Colors::RED, Rect::new(0.0, 0.0, 32.0, 32.0));
    fill(&mut context, Colors::RED, Rect::new(16.0, 0.0, 32.0, 32.0));
    context.pop_compositing_layer();
    fill(&mut context, Colors::BLUE, Rect::new(0.0, 40.0, 16.0, 16.0));
    context.finish();
    let image = pixels(&target.get_picture());
    assert_eq!(pixel(&image, 8, 8), pixel(&image, 24, 8));
    assert!((127..=128).contains(&pixel(&image, 24, 8)[3]));
    assert_eq!([0, 0, 255, 255], pixel(&image, 8, 48));
    assert_eq!(
        vec!["Isolated compositing layer"],
        context.audit().raster_reasons
    );
    assert!(context.audit().errors.is_empty());
}

#[test]
fn effects_are_forwarded_and_diagnostics_are_deduplicated() {
    let (target, mut context) = recording();
    for x in [12.0, 40.0] {
        let effect = context.as_drawing_context_impl_with_effects().unwrap();
        effect.push_effect(None, &ImmutableBlurEffect::new(8.0));
        fill(effect, Colors::RED, Rect::new(x, 20.0, 8.0, 8.0));
        effect.pop_effect();
    }
    context.finish();
    assert_eq!(
        vec!["Image or custom shader effect"],
        context.audit().raster_reasons
    );
    assert!(context.audit().errors.is_empty());
    let image = pixels(&target.get_picture());
    assert!(
        pixel(&image, 10, 23)[3] > 0,
        "the blur must extend beyond the original rectangle"
    );
    assert!(
        pixel(&image, 15, 23)[3] < 255,
        "the blur must actually reach the recording"
    );
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ExportShader<const KIND: u8>;

impl<const KIND: u8> ferroui_base::media::effects::CustomEffect for ExportShader<KIND> {
    const NAME: &'static str = match KIND {
        0 => "svg-test-missing-shader",
        1 => "svg-test-invalid-shader",
        _ => "svg-test-green-shader",
    };

    fn for_each_parameter(
        &self,
        _: &mut dyn FnMut(&'static str, ferroui_base::media::effects::EffectParameterValue),
    ) {
    }

    fn set_parameter(
        &mut self,
        _: &str,
        _: ferroui_base::media::effects::EffectParameterValue,
    ) -> bool {
        false
    }

    fn shader() -> Option<ferroui_base::media::effects::ShaderEffectDescription> {
        use ferroui_base::media::effects::ShaderEffectDescription;
        match KIND {
            0 => None,
            1 => Some(ShaderEffectDescription::content_sampling("invalid SkSL program", "")),
            _ => Some(ShaderEffectDescription::content_sampling(
                "uniform float2 layer_size; uniform float layer_scale; uniform shader content;\n\
                 half4 main(float2 coord) { half a = content.eval(coord).a; return half4(0.0, a, 0.0, a); }",
                "",
            )),
        }
    }
}

#[test]
fn unsupported_custom_effects_are_errors_instead_of_unchanged_content_fallback() {
    use ferroui_base::media::effects::ImmutableCustomEffect;
    let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
    for effect in [
        ImmutableCustomEffect::new(ExportShader::<0>),
        ImmutableCustomEffect::new(ExportShader::<1>),
    ] {
        let target = PictureRenderTarget::new(None, None, Vector::new(96.0, 96.0));
        let mut raw = target.create_export_drawing_context(Size::new(64.0, 64.0), false);
        assert!(!raw.try_push_custom_effect(Some(bounds), &effect));
        raw.dispose();

        let (target, mut context) = recording();
        context.push_effect(Some(bounds), &effect);
        fill(&mut context, Colors::RED, bounds);
        context.pop_effect();
        // A failed push has no underlying state to pop; later drawing still works.
        fill(&mut context, Colors::BLUE, Rect::new(40.0, 0.0, 16.0, 16.0));
        context.finish();
        let audit = context.audit();
        assert_eq!(1, audit.errors.len(), "{audit:?}");
        assert!(audit.errors[0].contains(effect.name()), "{audit:?}");
        assert!(audit.errors[0].contains("cannot execute"), "{audit:?}");
        assert_eq!(
            [0, 0, 255, 255],
            pixel(&pixels(&target.get_picture()), 44, 8)
        );
    }
}

#[test]
fn executable_custom_shader_reaches_the_recording_and_restores_siblings() {
    use ferroui_base::media::effects::ImmutableCustomEffect;
    let (target, mut context) = recording();
    let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
    context.push_effect(Some(bounds), &ImmutableCustomEffect::new(ExportShader::<2>));
    fill(&mut context, Colors::RED, bounds);
    context.pop_effect();
    fill(&mut context, Colors::BLUE, Rect::new(40.0, 0.0, 16.0, 16.0));
    context.finish();
    assert!(context.audit().errors.is_empty(), "{:?}", context.audit());
    assert_eq!(
        vec!["Image or custom shader effect"],
        context.audit().raster_reasons
    );
    let image = pixels(&target.get_picture());
    assert_eq!([0, 255, 0, 255], pixel(&image, 8, 8));
    assert_eq!([0, 0, 255, 255], pixel(&image, 44, 8));
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ExportImageShader {
    page: ferroui_base::media::effects::EffectImage,
}

impl ferroui_base::media::effects::CustomEffect for ExportImageShader {
    const NAME: &'static str = "svg-test-image-shader";
    fn for_each_parameter(
        &self,
        visit: &mut dyn FnMut(&'static str, ferroui_base::media::effects::EffectParameterValue),
    ) {
        visit("page", self.page.clone().into());
    }
    fn set_parameter(
        &mut self,
        _: &str,
        _: ferroui_base::media::effects::EffectParameterValue,
    ) -> bool {
        false
    }
    fn shader() -> Option<ferroui_base::media::effects::ShaderEffectDescription> {
        Some(
            ferroui_base::media::effects::ShaderEffectDescription::overlay(
                "uniform float2 layer_size; uniform float layer_scale; uniform shader page;\n\
             half4 main(float2 coord) { return page.eval(coord); }",
                "",
                BitmapBlendingMode::SourceOver,
            )
            .with_children(["page"]),
        )
    }
}

struct UnreadableBitmap;

impl IBitmapImpl for UnreadableBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }
    fn pixel_size(&self) -> PixelSize {
        PixelSize::new(2, 2)
    }
    fn version(&self) -> i32 {
        1
    }
    fn save(
        &self,
        _: &mut dyn std::io::Write,
        _: &ferroui_base::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "test image has no CPU pixels",
        ))
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn shader_images_distinguish_empty_and_readable_inputs_from_silently_missing_pixels() {
    use ferroui_base::media::effects::{EffectImage, ImmutableCustomEffect};
    use std::sync::Arc;
    let mut source = skia_safe::surfaces::raster_n32_premul((2, 2)).unwrap();
    source.canvas().clear(skia_safe::Color::BLUE);
    let readable = EffectImage::from_impl(Arc::new(ferroui_skia::ImmutableBitmap::from_image(
        source.image_snapshot(),
        None,
    )));
    for (page, expected) in [
        (EffectImage::default(), Some([255, 0, 0, 255])),
        (readable, Some([0, 0, 255, 255])),
        (EffectImage::from_impl(Arc::new(UnreadableBitmap)), None),
    ] {
        let (target, mut context) = recording();
        let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
        context.push_effect(
            Some(bounds),
            &ImmutableCustomEffect::new(ExportImageShader { page }),
        );
        fill(&mut context, Colors::RED, bounds);
        context.pop_effect();
        context.finish();
        if let Some(expected) = expected {
            assert!(context.audit().errors.is_empty(), "{:?}", context.audit());
            assert_eq!(expected, pixel(&pixels(&target.get_picture()), 16, 16));
        } else {
            assert!(
                context
                    .audit()
                    .errors
                    .iter()
                    .any(|error| error.contains("Shader image")),
                "{:?}",
                context.audit()
            );
        }
    }
}

#[test]
fn opacity_mask_preserves_alpha_and_siblings() {
    let (target, mut context) = recording();
    let mask = ImmutableSolidColorBrush::new(Color::from_argb(128, 255, 255, 255));
    context.push_opacity_mask(&mask, Rect::new(0.0, 0.0, 32.0, 32.0));
    fill(&mut context, Colors::RED, Rect::new(0.0, 0.0, 32.0, 32.0));
    context.pop_opacity_mask();
    fill(&mut context, Colors::BLUE, Rect::new(40.0, 0.0, 16.0, 16.0));
    context.finish();
    let image = pixels(&target.get_picture());
    assert_eq!(128, pixel(&image, 8, 8)[3]);
    assert_eq!([0, 0, 255, 255], pixel(&image, 48, 8));
    assert!(context.audit().errors.is_empty());
    assert!(!context.audit().raster_reasons.is_empty());
}

#[test]
fn native_lease_drawing_is_captured_at_disposal_and_classified() {
    let (target, mut context) = recording();
    context.set_transform(Matrix::create_translation(12.0, 8.0));
    let feature = context
        .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
        .unwrap();
    let feature = feature
        .downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>()
        .unwrap();
    let lease = feature.lease();
    assert!(lease.sk_surface().is_none());
    lease.with_sk_canvas(&mut |canvas| {
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::RED);
        canvas.draw_rect(skia_safe::Rect::from_xywh(0.0, 0.0, 12.0, 12.0), &paint);
    });
    lease.dispose();
    context.finish();
    let image = pixels(&target.get_picture());
    assert_eq!([255, 0, 0, 255], pixel(&image, 16, 12));
    assert_eq!([0, 0, 0, 0], pixel(&image, 4, 4));
    assert_eq!(1, context.audit().raster_reasons.len());
    assert!(context.audit().errors.is_empty());
}

#[test]
fn original_recordings_still_do_not_advertise_a_native_lease() {
    let target = PictureRenderTarget::new(None, None, Vector::new(96.0, 96.0));
    let mut context = target.create_drawing_context(Size::new(16.0, 16.0), false);
    assert!(context
        .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
        .is_none());
    context.dispose();
}

#[test]
fn skia_layer_blits_reach_the_underlying_context_without_a_downcast_panic() {
    let (target, mut context) = recording();
    let layer = context.create_layer(PixelSize::new(64, 64));
    let mut child = layer.create_drawing_context();
    child.clear(Colors::TRANSPARENT);
    fill(&mut *child, Colors::BLUE, Rect::new(4.0, 4.0, 16.0, 16.0));
    child.dispose();
    layer.blit(&mut context);
    context.finish();
    assert_eq!(
        [0, 0, 255, 255],
        pixel(&pixels(&target.get_picture()), 8, 8)
    );
    assert!(!context.audit().raster_reasons.is_empty());
    assert!(context.audit().errors.is_empty());
}

#[test]
fn invalid_inputs_and_unbalanced_state_fail_without_calling_skia_with_nan() {
    let (_, mut context) = recording();
    context.set_transform(Matrix::create_translation(f64::NAN, 0.0));
    context.push_clip(Rect::new(0.0, 0.0, f64::INFINITY, 32.0));
    context.pop_clip();
    context.pop_opacity();
    context.finish();
    assert_eq!(3, context.audit().errors.len());
    assert_eq!(Matrix::IDENTITY, context.transform());
}

#[test]
fn rejected_compositing_layer_does_not_change_stack_or_audit() {
    let (_, mut context) = recording();
    assert!(!context.try_push_compositing_layer(
        BitmapBlendingMode::SourceOver,
        2.0,
        Rect::new(0.0, 0.0, 32.0, 32.0)
    ));
    context.finish();
    assert!(context.audit().raster_reasons.is_empty());
    assert!(context.audit().errors.is_empty());
}

#[test]
fn offset_recording_bounds_do_not_change_the_drawing_transform() {
    let target = PictureRenderTarget::new(None, None, Vector::new(192.0, 144.0));
    let mut context =
        target.create_export_drawing_context_with_bounds(Rect::new(-10.0, 20.0, 40.0, 30.0), true);
    assert_eq!(Matrix::IDENTITY, context.transform());
    fill(&mut context, Colors::RED, Rect::new(-8.0, 22.0, 4.0, 4.0));
    context.dispose();
    assert_eq!(
        skia_safe::Rect::new(-20.0, 30.0, 60.0, 75.0),
        target.get_picture().cull_rect()
    );
}

#[test]
fn clearing_a_drawn_recording_is_not_mistaken_for_a_source_over_fill() {
    let (target, mut context) = recording();
    fill(&mut context, Colors::RED, Rect::new(0.0, 0.0, 32.0, 32.0));
    context.clear(Colors::TRANSPARENT);
    context.finish();
    assert!(!context.audit().raster_reasons.is_empty());
    assert_eq!([0, 0, 0, 0], pixel(&pixels(&target.get_picture()), 8, 8));
}

#[test]
fn completion_callback_runs_once_after_the_picture_is_available() {
    let target = Rc::new(PictureRenderTarget::new(
        None,
        None,
        Vector::new(96.0, 96.0),
    ));
    let completed = Rc::new(RefCell::new(Vec::new()));
    let output = completed.clone();
    let recorded = target.clone();
    let mut context = SvgDrawingContext::new(Box::new(
        target.create_export_drawing_context(Size::new(64.0, 64.0), false),
    ))
    .on_finished(move |audit| {
        let picture = recorded.get_picture();
        output
            .borrow_mut()
            .push((audit, pixel(&pixels(&picture), 8, 8)));
    });
    fill(&mut context, Colors::BLUE, Rect::new(0.0, 0.0, 16.0, 16.0));
    context.finish();
    context.finish();
    drop(context);
    let completed = completed.borrow();
    assert_eq!(1, completed.len());
    assert_eq!(1, completed[0].0.draw_calls);
    assert_eq!([0, 0, 255, 255], completed[0].1);
}

#[test]
fn dropping_the_adapter_finalizes_the_recording() {
    let completed = Rc::new(std::cell::Cell::new(0));
    let callback_count = completed.clone();
    let (target, context) = recording();
    {
        let mut context =
            context.on_finished(move |_| callback_count.set(callback_count.get() + 1));
        fill(&mut context, Colors::RED, Rect::new(0.0, 0.0, 16.0, 16.0));
    }
    assert_eq!(1, completed.get());
    assert_eq!(
        [255, 0, 0, 255],
        pixel(&pixels(&target.get_picture()), 8, 8)
    );
}

#[test]
fn reflected_and_repeating_gradients_request_fallback_but_padded_gradients_remain_vector() {
    use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
    let stops = [
        ImmutableGradientStop::new(0.0, Colors::RED),
        ImmutableGradientStop::new(1.0, Colors::BLUE),
    ];
    for spread in [
        GradientSpreadMethod::Pad,
        GradientSpreadMethod::Reflect,
        GradientSpreadMethod::Repeat,
    ] {
        let (_, mut context) = recording();
        let brush =
            ImmutableLinearGradientBrush::new(&stops, 1.0, None, None, spread, None, None, None);
        context.draw_rectangle(
            Some(&brush),
            None,
            RoundedRect::from_rect(Rect::new(0.0, 0.0, 32.0, 32.0)),
            &BoxShadows::default(),
        );
        context.finish();
        assert!(context.audit().errors.is_empty());
        assert_eq!(
            spread == GradientSpreadMethod::Pad,
            context.audit().raster_reasons.is_empty()
        );
    }
}

#[test]
fn affine_gradient_transforms_and_elliptic_radial_gradients_are_preserved_as_vectors() {
    use ferroui_base::media::immutable::{
        ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableRadialGradientBrush,
        ImmutableTransform,
    };
    let stops = [
        ImmutableGradientStop::new(0.0, Colors::RED),
        ImmutableGradientStop::new(1.0, Colors::BLUE),
    ];
    let (target, mut context) = recording();
    let linear = ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        Some(Rc::new(ImmutableTransform::new(
            Matrix::create_translation(4.0, 2.0),
        ))),
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        None,
    );
    context.set_transform(Matrix::create_translation(2.0, 3.0));
    context.draw_rectangle(
        Some(&linear),
        None,
        RoundedRect::from_rect(Rect::new(0.0, 0.0, 32.0, 16.0)),
        &BoxShadows::default(),
    );
    let radial = ImmutableRadialGradientBrush::from_stops(&stops);
    context.draw_rectangle(
        Some(&radial),
        None,
        RoundedRect::from_rect(Rect::new(0.0, 24.0, 48.0, 24.0)),
        &BoxShadows::default(),
    );
    context.finish();
    assert!(
        context.audit().raster_reasons.is_empty(),
        "{:?}",
        context.audit()
    );
    assert!(context.audit().errors.is_empty());
    let canvas = skia_safe::svg::Canvas::new(
        skia_safe::Rect::from_wh(64.0, 64.0),
        skia_safe::svg::canvas::Flags::empty(),
    );
    canvas.draw_picture(target.get_picture(), None, None);
    let data = canvas.end();
    let svg = std::str::from_utf8(data.as_bytes()).unwrap();
    let parsed = roxmltree::Document::parse(svg).unwrap();
    let linear = parsed
        .descendants()
        .find(|node| node.has_tag_name("linearGradient"))
        .unwrap();
    let radial = parsed
        .descendants()
        .find(|node| node.has_tag_name("radialGradient"))
        .unwrap();
    assert!(linear.attribute("gradientTransform").is_some(), "{svg}");
    assert!(radial.attribute("gradientTransform").is_some(), "{svg}");
    assert!(
        parsed
            .descendants()
            .any(|node| node.attribute("transform").is_some()),
        "{svg}"
    );
}

#[test]
fn asymmetric_rounded_draws_stay_vector_but_asymmetric_clips_request_fallback() {
    let (_, mut context) = recording();
    let rounded = RoundedRect::from_radii(Rect::new(0.0, 0.0, 40.0, 40.0), 12.0, 4.0, 8.0, 0.0);
    context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        rounded,
        &BoxShadows::default(),
    );
    assert!(context.audit().raster_reasons.is_empty());
    context.push_clip_rounded(rounded);
    fill(&mut context, Colors::BLUE, Rect::new(0.0, 0.0, 40.0, 40.0));
    context.pop_clip();
    context.finish();
    assert_eq!(
        vec!["Rounded clip with unequal corner radii"],
        context.audit().raster_reasons
    );
    assert!(context.audit().errors.is_empty());
}

#[test]
fn glyph_outline_capability_accepts_whitespace_and_rejects_drawable_only_glyphs() {
    use ferroui_base::media::text_formatting::GlyphInfo;
    use skia_safe as sk;
    let mut builder = sk::utils::CustomTypefaceBuilder::new();
    let glyph_bounds = sk::Rect::from_xywh(0.0, -1.0, 0.5, 1.0);
    builder.set_glyph(1, 0.75, &sk::Path::rect(glyph_bounds, None));
    builder.set_glyph(2, 0.5, &sk::Path::new());
    let mut recorder = sk::PictureRecorder::new();
    let canvas = recorder.begin_recording(glyph_bounds, false);
    let mut paint = sk::Paint::default();
    paint.set_color(sk::Color::RED);
    canvas.draw_rect(glyph_bounds, &paint);
    let drawable = recorder.finish_recording_as_drawable().unwrap();
    builder.set_glyph(3, 0.75, (drawable, glyph_bounds));
    let font = sk::Font::new(builder.detach().unwrap(), 12.0);
    let outline = ferroui_skia::GlyphRunImpl::from_font(
        &font,
        12.0,
        &[GlyphInfo::new(1, 0, 9.0), GlyphInfo::new(2, 1, 6.0)],
        Point::new(0.0, 20.0),
    );
    assert!(
        outline.has_vector_outlines(),
        "an available empty whitespace outline remains vector"
    );
    assert!(
        outline.has_vector_outlines(),
        "the cached capability is stable"
    );
    let drawable = ferroui_skia::GlyphRunImpl::from_font(
        &font,
        12.0,
        &[GlyphInfo::new(3, 0, 9.0)],
        Point::new(0.0, 20.0),
    );
    assert!(!drawable.has_vector_outlines());
    let (_, mut context) = recording();
    context.draw_glyph_run(
        Some(&ImmutableSolidColorBrush::new(Colors::BLACK)),
        &drawable,
    );
    context.finish();
    assert!(context.audit().errors.is_empty());
    assert_eq!(
        vec!["Glyph run requires colour, bitmap, or unavailable glyph outlines"],
        context.audit().raster_reasons
    );
}

#[test]
fn initial_clear_ignores_the_transform_in_svg_and_preserves_it_for_following_draws() {
    let (target, mut context) = recording();
    let transform = Matrix::create_translation(12.0, 8.0);
    context.set_transform(transform);
    context.clear(Colors::RED);
    assert_eq!(transform, context.transform());
    fill(&mut context, Colors::BLUE, Rect::new(0.0, 0.0, 8.0, 8.0));
    context.finish();
    assert!(context.audit().raster_reasons.is_empty());
    assert!(context.audit().errors.is_empty());
    let picture = target.get_picture();
    let image = pixels(&picture);
    assert_eq!([255, 0, 0, 255], pixel(&image, 2, 2));
    assert_eq!([0, 0, 255, 255], pixel(&image, 14, 10));
    let canvas = skia_safe::svg::Canvas::new(
        skia_safe::Rect::from_wh(64.0, 64.0),
        skia_safe::svg::canvas::Flags::empty(),
    );
    canvas.draw_picture(&picture, None, None);
    let data = canvas.end();
    let svg = std::str::from_utf8(data.as_bytes()).unwrap();
    let parsed = roxmltree::Document::parse(svg).unwrap();
    let background = parsed
        .descendants()
        .find(|node| node.has_tag_name("rect") && node.attribute("width") == Some("64"))
        .expect("initial clear must remain a full-target SVG rectangle");
    assert!(
        background
            .ancestors()
            .all(|node| node.attribute("transform").is_none()),
        "a clear must not inherit the caller's transform: {svg}"
    );
}

#[test]
fn first_transparent_clear_under_a_dirty_clip_does_not_force_raster_output() {
    let (target, mut context) = recording();
    context.push_clip(Rect::new(8.0, 8.0, 16.0, 16.0));
    context.clear(Colors::TRANSPARENT);
    fill(&mut context, Colors::BLUE, Rect::new(0.0, 0.0, 64.0, 64.0));
    context.pop_clip();
    context.finish();
    assert_eq!(2, context.audit().draw_calls);
    assert!(
        context.audit().raster_reasons.is_empty(),
        "{:?}",
        context.audit()
    );
    let image = pixels(&target.get_picture());
    assert_eq!([0, 0, 255, 255], pixel(&image, 12, 12));
    assert_eq!([0, 0, 0, 0], pixel(&image, 2, 2));
}

#[test]
fn opaque_clear_of_offset_bounds_requires_fallback_and_covers_every_raster_edge() {
    use crate::{SvgDocument, SvgError, SvgExportOptions, SvgFidelity, SvgLayerOptions};
    let mut document = SvgDocument::new(Size::new(64.0, 64.0)).unwrap();
    document
        .record_layer(
            SvgLayerOptions::new("offset", "Offset clear")
                .with_bounds(Rect::new(-16.0, 8.0, 32.0, 24.0)),
            |context| context.clear(Colors::RED),
        )
        .unwrap();
    assert_eq!(
        vec!["Nontransparent clear of an offset recording"],
        document.layers()[0].audit().raster_reasons,
    );
    assert!(matches!(
        document.export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        }),
        Err(SvgError::VectorFidelity { .. })
    ));
    let options = SvgExportOptions {
        raster_scale: 1.0,
        ..Default::default()
    };
    assert_eq!(1, document.export(&options).unwrap().report.raster_layers);
    let png = crate::render::raster_layer(&document.layers()[0], &options).unwrap();
    let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&png))
        .unwrap()
        .make_raster_image(None, skia_safe::image::CachingHint::Allow)
        .unwrap();
    let pixels = image.peek_pixels().unwrap();
    assert_eq!((32, 24), (pixels.width(), pixels.height()));
    for point in [(0, 0), (31, 0), (0, 23), (31, 23)] {
        assert_eq!(skia_safe::Color::RED, pixels.get_color(point));
    }
}

#[test]
fn transparent_clear_after_unclassified_native_drawing_still_erases_it() {
    let (target, mut context) = recording();
    let feature = context
        .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
        .unwrap();
    let feature = feature
        .downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>()
        .unwrap();
    let lease = feature.lease();
    lease.with_sk_canvas(&mut |canvas| {
        canvas.clear(skia_safe::Color::RED);
    });
    lease.dispose();
    // The native drawing was not counted as an adapter primitive.
    assert_eq!(0, context.audit().draw_calls);
    context.clear(Colors::TRANSPARENT);
    context.finish();
    assert_eq!([0, 0, 0, 0], pixel(&pixels(&target.get_picture()), 12, 12));
    assert!(!context.audit().raster_reasons.is_empty());
}

#[test]
fn scene3d_capability_draws_pixels_or_reports_a_missing_executor() {
    use ferroui_base::media::SharedBrush;
    use ferroui_base::numerics::{Matrix4x4, Vector3};
    use ferroui_base::rendering::scene3d::{
        MeshData, Scene3DCamera, Scene3DCameraKind, Scene3DMaterialKind, Scene3DMaterialLayer,
        Scene3DModel, Scene3DSnapshot,
    };
    use std::sync::Arc;
    let bounds = Rect::new(0.0, 0.0, 64.0, 64.0);
    let material = Scene3DMaterialLayer {
        kind: Scene3DMaterialKind::Emissive,
        color: Colors::WHITE,
        ambient_color: Colors::WHITE,
        specular_power: 0.0,
        brush: SharedBrush::from_brush(&ImmutableSolidColorBrush::new(Colors::RED)),
    };
    let mut scene = Scene3DSnapshot::empty();
    scene.viewport = bounds;
    scene.camera = Some(Scene3DCamera {
        kind: Scene3DCameraKind::Matrix,
        view: Matrix4x4::IDENTITY,
        projection: Matrix4x4::IDENTITY,
        position: Vector3::new(0.0, 0.0, 2.0),
        near_plane_distance: 0.0,
        far_plane_distance: 1.0,
    });
    scene.models.push(Scene3DModel {
        world: Matrix4x4::IDENTITY,
        mesh: Arc::new(MeshData {
            positions: vec![
                Vector3::new(-0.8, -0.8, 0.5),
                Vector3::new(0.8, -0.8, 0.5),
                Vector3::new(0.0, 0.8, 0.5),
            ],
            normals: vec![Vector3::new(0.0, 0.0, 1.0); 3],
            indices: vec![0, 1, 2],
            ..Default::default()
        }),
        front: vec![material.clone()],
        back: vec![material],
        world_bounds: None,
    });
    let frame = Scene3DFrame {
        scene: Arc::new(scene),
        bounds,
    };
    let (target, mut context) = recording();
    // Cargo may unify Skia's media3d feature through another workspace crate.
    // Observe the underlying renderer capability, not only this crate's cfg.
    let available = context
        .inner
        .as_drawing_context_impl_with_scene_3d()
        .is_some();
    #[cfg(feature = "media3d")]
    assert!(available, "ferroui-svg/media3d must enable Skia's executor");
    let drawn = context
        .as_drawing_context_impl_with_scene_3d()
        .unwrap()
        .draw_scene_3d(&frame);
    context.finish();
    let image = pixels(&target.get_picture());
    if available {
        assert!(drawn, "the media3d executor must reach the recording");
        assert!(context.audit().errors.is_empty(), "{:?}", context.audit());
        assert_eq!(1, context.audit().embedded_images);
        assert_eq!([255, 0, 0, 255], pixel(&image, 32, 32));
    } else {
        assert!(!drawn);
        assert!(context
            .audit()
            .errors
            .iter()
            .any(|error| error.contains("media3d")));
        assert_eq!([0, 0, 0, 0], pixel(&image, 32, 32));
    }
}
