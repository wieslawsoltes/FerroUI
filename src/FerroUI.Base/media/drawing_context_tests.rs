use crate::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use crate::media::{
    BoxShadows, Colors, DrawingContext, IBrush, IPen, ImmediateDrawingContext, PlatformDrawingContext, PushedState,
};
use crate::platform::IDrawingContextImpl;
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::rendering::ImmediateRenderer;
use crate::*;
use std::rc::Rc;

fn red() -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(Colors::RED))
}

fn pen(thickness: f64) -> Rc<dyn IPen> {
    Rc::new(ImmutablePen::with_brush(Some(Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE))), thickness))
}

fn quiet(log: &DrawingLog) -> MockDrawingContextImpl {
    let mut platform_impl = MockDrawingContextImpl::new(log.clone());
    platform_impl.log_transforms = false;
    platform_impl
}

#[test]
fn pushed_states_are_popped_in_reverse_order_on_dispose() {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    let _clip = context.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
    let _opacity = context.push_opacity(0.5);
    let _transform = context.push_transform(Matrix::create_translation(1.0, 2.0));
    context.dispose();
    assert_eq!(
        log.entries(),
        ["PushClip 0, 0, 10, 10", "PushOpacity 0.5", "PopOpacity", "PopClip"]
    );
}

#[test]
fn borrowed_core_is_not_disposed_and_owned_core_is() {
    let log = DrawingLog::new();
    {
        let mut platform_impl = quiet(&log);
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let context = DrawingContext::new(&mut core);
        context.dispose();
    }
    assert!(log.entries().is_empty());

    let context = DrawingContext::owned(Box::new(PlatformDrawingContext::new(Box::new(quiet(&log)))));
    drop(context);
    assert_eq!(log.entries(), ["Dispose"]);
}

#[test]
#[should_panic(expected = "Wrong Push/Pop state order")]
fn popping_out_of_order_panics() {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    let first = context.push_opacity(0.5);
    let _second = context.push_opacity(0.25);
    context.pop(first);
}

#[test]
fn popping_none_does_nothing() {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    context.pop(PushedState::NONE);
    let state = context.push_opacity(0.5);
    context.pop(PushedState::default());
    context.pop(state);
    assert_eq!(log.entries(), ["PushOpacity 0.5", "PopOpacity"]);
}

#[test]
fn scope_pops_when_dropped() {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    {
        let state = context.push_clip(Rect::new(0.0, 0.0, 5.0, 5.0));
        let mut scope = context.scope(state);
        scope.fill_rectangle(&red(), Rect::new(0.0, 0.0, 1.0, 1.0), 0.0);
    }
    context.draw_ellipse(Some(&red()), None, Rect::new(0.0, 0.0, 2.0, 2.0));
    assert_eq!(
        log.entries(),
        [
            "PushClip 0, 0, 5, 5",
            "DrawRectangle Red none 0, 0, 1, 1 shadows=0",
            "PopClip",
            "DrawEllipse Red none 0, 0, 2, 2"
        ]
    );
}

#[test]
fn push_transform_prepends_to_the_current_transform_and_pop_restores_it() {
    let log = DrawingLog::new();
    let mut platform_impl = MockDrawingContextImpl::new(log.clone());
    platform_impl.set_transform(Matrix::create_scale(2.0, 2.0));
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    let state = context.push_transform(Matrix::create_translation(10.0, 0.0));
    context.pop(state);
    drop(context);
    drop(core);
    assert_eq!(platform_impl.transform(), Matrix::create_scale(2.0, 2.0));
    let expected = Matrix::create_translation(10.0, 0.0) * Matrix::create_scale(2.0, 2.0);
    assert_eq!(log.entries()[1], format!("SetTransform {expected}"));
}

#[test]
fn invisible_pens_and_missing_brushes_draw_nothing() {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    context.draw_line(&pen(0.0), Point::new(0.0, 0.0), Point::new(1.0, 1.0));
    context.draw_rectangle(None, Some(&pen(0.0)), rect, 0.0, 0.0, &BoxShadows::default());
    context.draw_ellipse(None, None, rect);
    context.draw_ellipse_at(None, Some(&pen(0.0)), Point::new(0.0, 0.0), 1.0, 1.0);
    assert!(log.entries().is_empty());

    context.draw_line(&pen(1.0), Point::new(0.0, 0.0), Point::new(1.0, 1.0));
    context.draw_ellipse_at(Some(&red()), None, Point::new(5.0, 5.0), 2.0, 3.0);
    assert_eq!(
        log.entries(),
        ["DrawLine Blue@1 0, 0 1, 1", "DrawEllipse Red none 3, 2, 4, 6"]
    );
}

#[test]
fn rectangle_radii_are_clamped_to_half_the_size() {
    struct Capture(Vec<RoundedRect>);
    impl crate::media::IDrawingContextCore for Capture {
        fn draw_line_core(&mut self, _: &Rc<dyn IPen>, _: Point, _: Point) {}
        fn draw_geometry_impl_core(
            &mut self,
            _: Option<&Rc<dyn IBrush>>,
            _: Option<&Rc<dyn IPen>>,
            _: &Rc<dyn crate::platform::IGeometryImpl>,
        ) {
        }
        fn draw_rectangle_core(
            &mut self,
            _: Option<&Rc<dyn IBrush>>,
            _: Option<&Rc<dyn IPen>>,
            rrect: RoundedRect,
            _: &BoxShadows,
        ) {
            self.0.push(rrect);
        }
        fn draw_ellipse_core(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: Rect) {}
        fn draw_bitmap(&mut self, _: &Rc<dyn crate::platform::IBitmapImpl>, _: f64, _: Rect, _: Rect) {}
        fn custom(&mut self, _: &Rc<dyn crate::rendering::scene_graph::ICustomDrawOperation>) {}
        fn draw_glyph_run(&mut self, _: Option<&Rc<dyn IBrush>>, _: &Rc<crate::media::GlyphRun>) {}
        fn push_clip_core(&mut self, _: Rect) {}
        fn push_rounded_clip_core(&mut self, _: RoundedRect) {}
        fn push_geometry_clip_core(&mut self, _: &Ref<crate::media::Geometry>) {}
        fn push_opacity_core(&mut self, _: f64) {}
        fn push_opacity_mask_core(&mut self, _: &Rc<dyn IBrush>, _: Rect) {}
        fn push_transform_core(&mut self, _: Matrix) {}
        fn push_render_options_core(&mut self, _: crate::media::RenderOptions) {}
        fn push_text_options_core(&mut self, _: crate::media::TextOptions) {}
        fn push_effect_core(&mut self, _: &Rc<dyn crate::media::IEffect>, _: Rect) {}
        fn pop_clip_core(&mut self) {}
        fn pop_geometry_clip_core(&mut self) {}
        fn pop_opacity_core(&mut self) {}
        fn pop_opacity_mask_core(&mut self) {}
        fn pop_transform_core(&mut self) {}
        fn pop_render_options_core(&mut self) {}
        fn pop_text_options_core(&mut self) {}
        fn pop_effect_core(&mut self) {}
        fn dispose_core(&mut self) {}
    }

    let mut capture = Capture(Vec::new());
    {
        let mut context = DrawingContext::new(&mut capture);
        context.draw_rectangle(Some(&red()), None, Rect::new(0.0, 0.0, 10.0, 4.0), 20.0, 20.0, &BoxShadows::default());
    }
    assert_eq!(capture.0.len(), 1);
    assert_eq!(capture.0[0].radii_top_left, Vector::new(5.0, 2.0));
}

#[test]
fn immediate_context_transform_container_composes_and_restores() {
    let log = DrawingLog::new();
    let mut platform_impl = MockDrawingContextImpl::new(log.clone());
    platform_impl.set_transform(Matrix::create_translation(100.0, 0.0));
    log.clear();
    {
        let mut context = ImmediateDrawingContext::borrowed(&mut platform_impl);
        let scale = context.push_set_transform(Matrix::create_scale(2.0, 2.0));
        let container = context.push_transform_container();
        assert_eq!(context.current_transform(), Matrix::IDENTITY);
        let inner = context.push_post_transform(Matrix::create_translation(1.0, 1.0));
        context.pop(inner);
        context.pop(container);
        assert_eq!(context.current_transform(), Matrix::create_scale(2.0, 2.0));
        context.pop(scale);
        context.dispose();
    }
    let outer = Matrix::create_scale(2.0, 2.0) * Matrix::create_translation(100.0, 0.0);
    let entries = log.entries();
    assert_eq!(entries[0], format!("SetTransform {outer}"));
    assert_eq!(entries[1], format!("SetTransform {}", Matrix::create_translation(1.0, 1.0) * outer));
    assert_eq!(platform_impl.transform(), Matrix::create_translation(100.0, 0.0));
}

#[test]
#[should_panic(expected = "Wrong Push/Pop state order")]
fn immediate_context_checks_pop_order() {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut context = ImmediateDrawingContext::borrowed(&mut platform_impl);
    let first = context.push_clip(Rect::new(0.0, 0.0, 1.0, 1.0));
    let _second = context.push_clip(Rect::new(0.0, 0.0, 1.0, 1.0));
    context.pop(first);
}

// --- immediate renderer ---------------------------------------------------

#[repr(C)]
struct Painted {
    base: Visual,
}

ferro_class!(Painted: Visual);
ferro_impl_classes!(Painted: FerroObjectImpl, StyledElementImpl);

impl VisualImpl for Painted {
    fn render(this: &Self, context: &mut DrawingContext) {
        context.fill_rectangle(&red(), Rect::from_size(this.bounds().size()), 0.0);
    }
}

impl Painted {
    fn new(bounds: Rect) -> Ref<Self> {
        let visual = instantiate(Self { base: Visual::construct() });
        visual.set_bounds(bounds);
        visual
    }
}

fn render_tree(root: &Ref<Painted>) -> Vec<String> {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    ImmediateRenderer::render(&mut context, &root.clone().upcast());
    context.dispose();
    log.entries().into_iter().filter(|e| e.starts_with("Draw") || e.contains("Clip")).collect()
}

#[test]
fn immediate_renderer_renders_children_in_z_order_and_skips_invisible_ones() {
    let root = Painted::new(Rect::new(0.0, 0.0, 100.0, 100.0));
    let a = Painted::new(Rect::new(0.0, 0.0, 10.0, 10.0));
    let b = Painted::new(Rect::new(0.0, 0.0, 20.0, 20.0));
    let c = Painted::new(Rect::new(0.0, 0.0, 30.0, 30.0));
    root.visual_children().add(a.clone().upcast());
    root.visual_children().add(b.clone().upcast());
    root.visual_children().add(c.clone().upcast());
    b.set_is_visible(false);

    let drawn = render_tree(&root);
    assert_eq!(
        drawn,
        [
            "PushClip 0, 0, 100, 100",
            "DrawRectangle Red none 0, 0, 100, 100 shadows=0",
            "DrawRectangle Red none 0, 0, 10, 10 shadows=0",
            "DrawRectangle Red none 0, 0, 30, 30 shadows=0",
            "PopClip"
        ]
    );
}

#[test]
fn immediate_renderer_culls_visuals_outside_the_clip() {
    let root = Painted::new(Rect::new(0.0, 0.0, 100.0, 100.0));
    root.set_clip_to_bounds(true);
    let outside = Painted::new(Rect::new(200.0, 0.0, 10.0, 10.0));
    let inside = Painted::new(Rect::new(90.0, 90.0, 20.0, 20.0));
    root.visual_children().add(outside.upcast());
    root.visual_children().add(inside.upcast());

    let drawn = render_tree(&root);
    assert_eq!(drawn.iter().filter(|e| e.starts_with("DrawRectangle")).count(), 2);
    assert!(drawn.contains(&"DrawRectangle Red none 0, 0, 20, 20 shadows=0".to_string()));
    assert!(!drawn.contains(&"DrawRectangle Red none 0, 0, 10, 10 shadows=0".to_string()));
}

#[test]
fn immediate_renderer_skips_transparent_visuals() {
    let root = Painted::new(Rect::new(0.0, 0.0, 100.0, 100.0));
    root.set_opacity(0.0);
    assert!(render_tree(&root).iter().all(|e| !e.starts_with("DrawRectangle")));
}

// --- images, drawings, effects, acrylic --------------------------------------

use crate::media::effects::{BlurEffect, EffectExtensions, IEffect};
use crate::media::imaging::{Bitmap, CroppedBitmap, RenderTargetBitmap};
use crate::media::{
    Drawing, DrawingImage, GeometryDrawing, IImage, ImageDrawing, ImmutableExperimentalAcrylicMaterial,
    RectangleGeometry,
};
use crate::platform::{AlphaFormat, PixelFormat};
use crate::rendering::testing::MockPlatformRenderInterface;

/// A 4x2 pixel bitmap at 96 DPI, created through the mock render interface.
fn bitmap() -> Rc<Bitmap> {
    Rc::new(Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &[0u8; 32],
        PixelSize::new(4, 2),
        Vector::new(96.0, 96.0),
        16,
    ))
}

/// Runs `draw` on a drawing context over a mock platform context and
/// returns what reached the platform context.
fn drawn(draw: impl FnOnce(&mut DrawingContext<'_>)) -> Vec<String> {
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    draw(&mut context);
    context.dispose();
    log.entries()
}

#[test]
fn draw_image_draws_a_bitmap_with_its_whole_area_as_source() {
    let (scope, _) = MockPlatformRenderInterface::install();
    let bitmap = bitmap();

    let entries = drawn(|context| context.draw_image(&*bitmap, Rect::new(10.0, 10.0, 8.0, 4.0)));
    assert_eq!(entries, ["DrawBitmap 1 0, 0, 4, 2 10, 10, 8, 4"]);

    let entries = drawn(|context| {
        context.draw_image_with_rects(&*bitmap, Rect::new(1.0, 0.0, 2.0, 2.0), Rect::new(0.0, 0.0, 2.0, 2.0))
    });
    assert_eq!(entries, ["DrawBitmap 1 1, 0, 2, 2 0, 0, 2, 2"]);

    scope.dispose();
}

#[test]
fn cropped_bitmap_draws_its_source_offset_by_the_crop_origin() {
    let (scope, _) = MockPlatformRenderInterface::install();
    let source: Rc<dyn IImage> = bitmap();
    let cropped = CroppedBitmap::with_source(source, PixelRect::new(1, 1, 2, 1));
    let image: Rc<dyn IImage> = cropped.clone().into();
    assert_eq!(Size::new(2.0, 1.0), image.size());

    let entries = drawn(|context| context.draw_image(&*image, Rect::new(0.0, 0.0, 20.0, 10.0)));
    assert_eq!(entries, ["DrawBitmap 1 1, 1, 2, 1 0, 0, 20, 10"]);

    // Without a source nothing is drawn.
    cropped.set_source(None);
    let entries =
        drawn(|context| cropped.draw(context, Rect::new(0.0, 0.0, 2.0, 1.0), Rect::new(0.0, 0.0, 20.0, 10.0)));
    assert!(entries.is_empty());

    scope.dispose();
}

fn geometry_drawing(rect: Rect) -> Ref<GeometryDrawing> {
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(red()));
    drawing.set_geometry(RectangleGeometry::with_rect(rect).upcast::<crate::media::Geometry>());
    drawing
}

#[test]
fn geometry_drawing_draws_its_geometry_and_image_drawing_its_image() {
    let (scope, _) = MockPlatformRenderInterface::install();

    let drawing = geometry_drawing(Rect::new(1.0, 2.0, 3.0, 4.0));
    let entries = drawn(|context| drawing.draw(context));
    assert_eq!(entries, ["DrawGeometry Red none 1, 2, 3, 4"]);

    // No geometry: nothing is drawn.
    let empty = GeometryDrawing::new();
    assert!(drawn(|context| empty.draw(context)).is_empty());

    let image_drawing = ImageDrawing::new();
    // No image, or an empty rectangle: nothing is drawn.
    image_drawing.set_rect(Rect::new(5.0, 5.0, 8.0, 4.0));
    assert!(drawn(|context| image_drawing.draw(context)).is_empty());
    let source: Rc<dyn IImage> = bitmap();
    image_drawing.set_image_source(Some(source));
    image_drawing.set_rect(Rect::default());
    assert!(drawn(|context| image_drawing.draw(context)).is_empty());

    image_drawing.set_rect(Rect::new(5.0, 5.0, 8.0, 4.0));
    let drawing: Ref<Drawing> = image_drawing.upcast();
    let entries = drawn(|context| drawing.draw(context));
    assert_eq!(entries, ["DrawBitmap 1 0, 0, 4, 2 5, 5, 8, 4"]);

    scope.dispose();
}

#[test]
fn drawing_image_clips_to_the_destination_and_maps_the_source_onto_it() {
    let (scope, _) = MockPlatformRenderInterface::install();
    let drawing = geometry_drawing(Rect::new(10.0, 20.0, 30.0, 40.0));
    let image = DrawingImage::with_drawing(drawing.upcast::<Drawing>());
    let handle: Rc<dyn IImage> = image.clone().into();
    assert_eq!(Size::new(30.0, 40.0), handle.size());

    let log = DrawingLog::new();
    let mut platform_impl = MockDrawingContextImpl::new(log.clone());
    {
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let mut context = DrawingContext::new(&mut core);
        context.draw_image(&*handle, Rect::new(100.0, 100.0, 60.0, 80.0));
        context.dispose();
    }
    // The drawing's bounds origin is moved to the destination origin, then
    // everything is scaled by destination size / source size.
    let transform = Matrix::create_translation(100.0 - 10.0, 100.0 - 20.0) * Matrix::create_scale(2.0, 2.0);
    assert_eq!(
        log.entries(),
        [
            "PushClip 100, 100, 60, 80".to_string(),
            format!("SetTransform {transform}"),
            "DrawGeometry Red none 10, 20, 30, 40".to_string(),
            format!("SetTransform {}", Matrix::IDENTITY),
            "PopClip".to_string(),
        ]
    );

    // An empty source or destination, an empty drawing and a missing
    // drawing draw nothing.
    let empty = Rect::new(1.0, 1.0, 0.0, 0.0);
    assert!(drawn(|context| image.draw(context, empty, Rect::new(0.0, 0.0, 1.0, 1.0))).is_empty());
    assert!(drawn(|context| image.draw(context, Rect::new(0.0, 0.0, 1.0, 1.0), empty)).is_empty());
    image.set_viewbox(Some(Rect::new(3.0, 3.0, 0.0, 0.0)));
    assert!(drawn(|context| context.draw_image(&*handle, Rect::new(0.0, 0.0, 1.0, 1.0))).is_empty());
    image.set_viewbox(None);
    image.set_drawing(None);
    assert!(drawn(|context| context.draw_image(&*handle, Rect::new(0.0, 0.0, 1.0, 1.0))).is_empty());

    scope.dispose();
}

#[test]
fn immediate_context_draws_bitmaps() {
    let (scope, _) = MockPlatformRenderInterface::install();
    let bitmap = bitmap();
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    {
        let mut context = ImmediateDrawingContext::borrowed(&mut platform_impl);
        context.draw_bitmap(&bitmap, Rect::new(0.0, 0.0, 40.0, 20.0));
        context.draw_bitmap_rects(&bitmap, Rect::new(2.0, 0.0, 2.0, 2.0), Rect::new(1.0, 1.0, 2.0, 2.0));
        context.draw_bitmap_impl(&*bitmap.platform_impl().item(), Rect::new(0.0, 0.0, 1.0, 1.0), Rect::default());
        context.dispose();
    }
    assert_eq!(
        log.entries(),
        [
            "DrawBitmap 1 0, 0, 4, 2 0, 0, 40, 20",
            "DrawBitmap 1 2, 0, 2, 2 1, 1, 2, 2",
            "DrawBitmap 1 0, 0, 1, 1 0, 0, 0, 0"
        ]
    );
    scope.dispose();
}

#[test]
fn push_effect_reaches_a_platform_context_with_effect_support_with_inflated_bounds() {
    let effect = BlurEffect::new();
    effect.set_radius(10.0);
    let effect: Rc<dyn IEffect> = effect.into();
    let content_bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
    let expected = content_bounds.inflate_thickness(EffectExtensions::get_effect_output_padding(Some(&*effect)));
    assert_ne!(expected, content_bounds);

    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    platform_impl.supports_effects = true;
    {
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let mut context = DrawingContext::new(&mut core);
        let state = context.push_effect(&effect, content_bounds);
        context.fill_rectangle(&red(), Rect::new(20.0, 20.0, 50.0, 50.0), 0.0);
        context.pop(state);
    }
    assert_eq!(
        log.entries(),
        [format!("PushEffect {expected}"), "DrawRectangle Red none 20, 20, 50, 50 shadows=0".to_string(), "PopEffect".to_string()]
    );

    // A platform context without effect support draws the content as is.
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    {
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let mut context = DrawingContext::new(&mut core);
        let state = context.push_effect(&effect, content_bounds);
        context.fill_rectangle(&red(), Rect::new(20.0, 20.0, 50.0, 50.0), 0.0);
        context.pop(state);
    }
    assert_eq!(log.entries(), ["DrawRectangle Red none 20, 20, 50, 50 shadows=0"]);
}

#[test]
fn acrylic_rectangle_uses_the_platform_support_or_the_fallback_color() {
    let material = crate::media::ExperimentalAcrylicMaterial::new();
    material.set_tint_color(Colors::BLUE);
    material.set_fallback_color(Colors::GREEN);
    let handle: Rc<dyn crate::media::IExperimentalAcrylicMaterial> = material.into();
    let material = ImmutableExperimentalAcrylicMaterial::new(&*handle);
    let rect = RoundedRect::from_rect(Rect::new(1.0, 2.0, 30.0, 40.0));

    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    platform_impl.supports_acrylic = true;
    PlatformDrawingContext::borrowed(&mut platform_impl).draw_rectangle_acrylic(&material, rect);
    assert_eq!(log.entries(), [format!("DrawRectangleWithMaterial {} 1, 2, 30, 40", material.tint_color())]);

    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    PlatformDrawingContext::borrowed(&mut platform_impl).draw_rectangle_acrylic(&material, rect);
    assert_eq!(log.entries(), ["DrawRectangle Green none 1, 2, 30, 40 shadows=0"]);
}

#[test]
fn render_target_bitmap_renders_a_visual_and_disposes_the_platform_context() {
    let (scope, render_interface) = MockPlatformRenderInterface::install();
    let bitmap = RenderTargetBitmap::new(PixelSize::new(100, 50));
    assert_eq!(Size::new(100.0, 50.0), bitmap.size());

    let root = Painted::new(Rect::new(0.0, 0.0, 100.0, 50.0));
    bitmap.render(&root.upcast());
    let entries: Vec<String> =
        render_interface.log().take().into_iter().filter(|e| !e.starts_with("SetTransform")).collect();
    assert_eq!(
        entries,
        [
            "Clear Transparent",
            "PushClip 0, 0, 100, 50",
            "PushOpacity 1",
            "DrawRectangle Red none 0, 0, 100, 50 shadows=0",
            "PopOpacity",
            "PopClip",
            "Dispose"
        ]
    );

    // Without clearing, and disposing by leaving the scope.
    {
        let mut context = bitmap.create_drawing_context_with_clear(false);
        context.fill_rectangle(&red(), Rect::new(0.0, 0.0, 1.0, 1.0), 0.0);
    }
    assert_eq!(render_interface.log().take(), ["DrawRectangle Red none 0, 0, 1, 1 shadows=0", "Dispose"]);

    let context = bitmap.create_drawing_context();
    context.dispose();
    assert_eq!(render_interface.log().take(), ["Clear Transparent", "Dispose"]);

    bitmap.dispose();
    scope.dispose();
}

// --- text, text options, acrylic through the drawing context ----------------

use crate::utilities::CultureInfo;
use crate::media::text_formatting::testing::{utf16, TextTestScope};
use crate::media::text_formatting::{TextLayout, TextLayoutOptions};
use crate::media::{
    Brushes, FlowDirection, FormattedText, GlyphRun, TextHintingMode, TextOptions, TextRenderingMode, Typeface,
};
use crate::rendering::composition::drawing::RenderDataDrawingContext;

fn black() -> Rc<dyn IBrush> {
    Brushes::black()
}

fn formatted(text: &str) -> FormattedText {
    FormattedText::new(
        text,
        CultureInfo::invariant_culture(),
        FlowDirection::LeftToRight,
        Typeface::default_typeface(),
        12.0,
        Some(black()),
    )
}

/// A glyph run of the default test font at an em size of 12: 6 per glyph
/// wide, from 9.6 above the baseline to 2.4 below it.
fn glyph_run(text: &str) -> Rc<GlyphRun> {
    let glyph_typeface = Typeface::default_typeface().glyph_typeface();
    let glyphs: Vec<u16> =
        text.chars().map(|c| glyph_typeface.character_to_glyph_map().get_glyph(c as i32)).collect();
    GlyphRun::from_glyph_indices(glyph_typeface, 12.0, utf16(text), &glyphs, None, 0)
}

/// Like [`drawn`], with the transform changes.
fn drawn_with_transforms(draw: impl FnOnce(&mut DrawingContext<'_>)) -> Vec<String> {
    let log = DrawingLog::new();
    let mut platform_impl = MockDrawingContextImpl::new(log.clone());
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    draw(&mut context);
    context.dispose();
    log.entries()
}

#[test]
fn draw_glyph_run_reaches_the_platform_context_unless_there_is_no_foreground() {
    let _scope = TextTestScope::new();
    let run = glyph_run("abc");
    let bounds = run.platform_impl().bounds();
    assert_eq!(bounds.size(), Size::new(18.0, 12.0));

    let entries = drawn(|context| {
        context.draw_glyph_run(None, &run);
        context.draw_glyph_run(Some(&black()), &run);
    });
    assert_eq!(entries, [format!("DrawGlyphRun Black {bounds}")]);
}

#[test]
fn draw_text_draws_every_line_translated_to_its_origin() {
    let _scope = TextTestScope::new();
    let text = formatted("ab\ncde");
    let line_height = text.height() / 2.0;

    let entries = drawn_with_transforms(|context| context.draw_text(&text, Point::new(10.0, 20.0)));

    let glyph_runs: Vec<&String> = entries.iter().filter(|e| e.starts_with("DrawGlyphRun")).collect();
    assert_eq!(glyph_runs.len(), 2);
    assert!(glyph_runs[0].starts_with("DrawGlyphRun Black ") && glyph_runs[0].ends_with(", 12, 12"));
    assert!(glyph_runs[1].ends_with(", 18, 12"));

    // Every run is drawn inside a translation to the origin of its line,
    // which is popped again: the context ends with its initial transform.
    let transforms: Vec<Matrix> = [Point::new(10.0, 20.0), Point::new(10.0, 20.0 + line_height)]
        .iter()
        .map(|origin| Matrix::create_translation(origin.x, origin.y))
        .collect();
    assert_eq!(
        entries.iter().filter(|e| !e.starts_with("DrawGlyphRun")).cloned().collect::<Vec<_>>(),
        [
            format!("SetTransform {}", transforms[0]),
            format!("SetTransform {}", Matrix::IDENTITY),
            format!("SetTransform {}", transforms[1]),
            format!("SetTransform {}", Matrix::IDENTITY),
        ]
    );
    let first_draw = entries.iter().position(|e| e.starts_with("DrawGlyphRun")).unwrap();
    assert_eq!(first_draw, 1);

    // Drawing again (the metrics are known now) draws the same.
    assert_eq!(drawn_with_transforms(|context| context.draw_text(&text, Point::new(10.0, 20.0))), entries);
}

#[test]
fn a_text_layout_draws_through_the_drawing_context_as_its_sink() {
    let _scope = TextTestScope::new();
    let layout = TextLayout::new(
        "abc",
        Typeface::default_typeface(),
        TextLayoutOptions { font_size: 12.0, foreground: Some(black()), ..TextLayoutOptions::default() },
    );

    let entries = drawn(|context| {
        let state = context.push_opacity(0.5);
        layout.draw(context, Point::new(3.0, 4.0));
        // The states the text pushed are popped: the opacity is the top one.
        context.pop(state);
    });
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0], "PushOpacity 0.5");
    assert!(entries[1].starts_with("DrawGlyphRun Black ") && entries[1].ends_with(", 18, 12"));
    assert_eq!(entries[2], "PopOpacity");
}

#[test]
#[should_panic(expected = "Wrong Push/Pop state order")]
fn a_sink_pop_that_does_not_match_a_transform_push_panics() {
    use crate::media::text_formatting::ITextDrawingSink;
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    let _opacity = context.push_opacity(0.5);
    ITextDrawingSink::pop_transform(&mut context);
}

#[test]
fn draw_text_is_recorded_and_replayed() {
    let _scope = TextTestScope::new();
    let text = formatted("abc");

    let direct = drawn(|context| context.draw_text(&text, Point::new(10.0, 20.0)));

    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        context.draw_text(&text, Point::new(10.0, 20.0));
    }
    let content = recorder
        .get_immediate_scene_brush_content(
            Rc::new(crate::media::immutable::ImmutableImageBrush::from_bitmap(None)),
            None,
            true,
        )
        .expect("something was drawn");
    // A transform scope around the glyph run.
    assert_eq!(content.with_stream(|stream| stream.opcode_length()), Some(3));
    let replayed = content
        .with_stream(|stream| {
            let log = DrawingLog::new();
            let mut platform_impl = quiet(&log);
            stream.replay(&mut platform_impl);
            log.entries()
        })
        .unwrap();
    assert_eq!(replayed, direct);
    assert_eq!(replayed.len(), 1);
}

#[test]
fn text_options_are_pushed_and_popped_in_order() {
    let options = TextOptions {
        text_rendering_mode: TextRenderingMode::Antialias,
        text_hinting_mode: TextHintingMode::Light,
        ..Default::default()
    };
    let entries = drawn(|context| {
        let render_options = context.push_render_options(Default::default());
        let text_options = context.push_text_options(options);
        let opacity = context.push_opacity(0.5);
        context.fill_rectangle(&red(), Rect::new(0.0, 0.0, 1.0, 1.0), 0.0);
        context.pop(opacity);
        context.pop(text_options);
        context.pop(render_options);
        // Left to the disposal of the context.
        let _ = context.push_text_options(TextOptions::default());
    });
    assert_eq!(
        entries,
        [
            "PushRenderOptions",
            "PushTextOptions Antialias Light Unspecified",
            "PushOpacity 0.5",
            "DrawRectangle Red none 0, 0, 1, 1 shadows=0",
            "PopOpacity",
            "PopTextOptions",
            "PopRenderOptions",
            "PushTextOptions Unspecified Unspecified Unspecified",
            "PopTextOptions",
        ]
    );
}

#[test]
fn immediate_renderer_pushes_the_text_options_of_a_visual_first() {
    let root = Painted::new(Rect::new(0.0, 0.0, 100.0, 100.0));
    let child = Painted::new(Rect::new(0.0, 0.0, 10.0, 10.0));
    root.visual_children().add(child.clone().upcast());

    let render = |root: &Ref<Painted>| -> Vec<String> {
        let log = DrawingLog::new();
        let mut platform_impl = quiet(&log);
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let mut context = DrawingContext::new(&mut core);
        ImmediateRenderer::render(&mut context, &root.clone().upcast());
        context.dispose();
        log.entries().into_iter().filter(|e| e.contains("Options") || e.contains("Opacity")).collect()
    };

    // Default text options are not pushed.
    assert_eq!(render(&root), ["PushOpacity 1", "PushOpacity 1", "PopOpacity", "PopOpacity"]);

    TextOptions::set_text_rendering_mode(&child, TextRenderingMode::Alias);
    crate::media::RenderOptions::set_edge_mode(&child, crate::media::EdgeMode::Aliased);
    assert_eq!(TextOptions::get_text_options(&child).text_rendering_mode, TextRenderingMode::Alias);
    assert_eq!(
        render(&root),
        [
            "PushOpacity 1",
            "PushTextOptions Alias Unspecified Unspecified",
            "PushRenderOptions",
            "PushOpacity 1",
            "PopOpacity",
            "PopRenderOptions",
            "PopTextOptions",
            "PopOpacity",
        ]
    );
}

#[test]
fn immediate_context_draws_a_glyph_run_reference_until_it_is_disposed() {
    let _scope = TextTestScope::new();
    let run = glyph_run("ab");
    let bounds = run.platform_impl().bounds();
    let reference = run.try_create_immutable_glyph_run_reference().expect("the run has a platform glyph run");

    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    let mut context = ImmediateDrawingContext::borrowed(&mut platform_impl);
    context.draw_glyph_run(&ImmutableSolidColorBrush::new(Colors::RED), &*reference);
    reference.dispose();
    context.draw_glyph_run(&ImmutableSolidColorBrush::new(Colors::RED), &*reference);
    context.dispose();
    assert_eq!(log.entries(), [format!("DrawGlyphRun Red {bounds}")]);
}

#[test]
fn acrylic_rectangle_is_reachable_through_the_drawing_context() {
    let material = crate::media::ExperimentalAcrylicMaterial::new();
    material.set_tint_color(Colors::BLUE);
    material.set_fallback_color(Colors::GREEN);
    let handle: Rc<dyn crate::media::IExperimentalAcrylicMaterial> = material.into();
    let material = ImmutableExperimentalAcrylicMaterial::new(&*handle);
    let rect = RoundedRect::from_rect(Rect::new(1.0, 2.0, 30.0, 40.0));

    // A platform core uses the acrylic support of the backend ...
    let log = DrawingLog::new();
    let mut platform_impl = quiet(&log);
    platform_impl.supports_acrylic = true;
    {
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let mut context = DrawingContext::new(&mut core);
        context.draw_rectangle_acrylic(&material, rect);
    }
    assert_eq!(log.entries(), [format!("DrawRectangleWithMaterial {} 1, 2, 30, 40", material.tint_color())]);

    // ... or the fallback color when the backend has none.
    assert_eq!(
        drawn(|context| context.draw_rectangle_acrylic(&material, rect)),
        ["DrawRectangle Green none 1, 2, 30, 40 shadows=0"]
    );

    // Any other core draws the fallback color: here it is recorded.
    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        context.draw_rectangle_acrylic(&material, rect);
    }
    let content = recorder
        .get_immediate_scene_brush_content(
            Rc::new(crate::media::immutable::ImmutableImageBrush::from_bitmap(None)),
            None,
            true,
        )
        .expect("something was drawn");
    let replayed = content
        .with_stream(|stream| {
            let log = DrawingLog::new();
            let mut platform_impl = quiet(&log);
            stream.replay(&mut platform_impl);
            log.entries()
        })
        .unwrap();
    assert_eq!(replayed, ["DrawRectangle Green none 1, 2, 30, 40 shadows=0"]);
}
