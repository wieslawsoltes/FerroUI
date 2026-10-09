//! Render tests of stage 6 of the design document: box shadows, effects,
//! scene brushes, render options, pixel formats and codecs.
//!
//! The tests of the Skia backend of these members (`tests.rs`,
//! `immutable_bitmap.rs` and `unit_tests/media/bitmap_save_tests.rs` of that
//! crate) with the same scenes and expectations, and tests of what this
//! backend does in its own way: which shadows are drawn in closed form, the
//! scene of its own an effect is recorded into under a clip or in a
//! rendering mode without filters.

use crate::scene::{
    IVelloSceneSink, VelloCpuSceneSink, VelloSceneCapabilities, VelloSceneFilter, VelloSceneFilterCapabilities,
    VelloScenePaint,
};
use crate::*;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadow, BoxShadows, Color, Colors, EdgeMode, RenderOptions};
use ferroui_base::platform::{
    IDrawingContextImpl, ILockedFramebuffer, IPlatformRenderInterface, IReadableBitmapImpl, IRenderTargetBitmapImpl,
    PixelFormat,
};
use ferroui_base::{PixelSize, Rect, RoundedRect, Vector};
use kurbo::{Affine, BezPath, Stroke};
use peniko::{BlendMode, Fill};
use std::cell::RefCell;
use std::rc::Rc;

const DPI: Vector = Vector::new(96.0, 96.0);

const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);
const BLUE: (u8, u8, u8, u8) = (0, 0, 255, 255);
const TRANSPARENT: (u8, u8, u8, u8) = (0, 0, 0, 0);

fn render_interface() -> PlatformRenderInterface {
    PlatformRenderInterface::default()
}

fn solid(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn aliased() -> RenderOptions {
    RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, w, h))
}

fn no_shadows() -> BoxShadows {
    BoxShadows::default()
}

fn read_framebuffer_pixel(framebuffer: &dyn ILockedFramebuffer, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let size = framebuffer.size();
    assert!(x >= 0 && y >= 0 && x < size.width && y < size.height);
    let offset = (y * framebuffer.row_bytes() + x * 4) as usize;
    let mut bytes = [0u8; 4];
    framebuffer.with_data(&mut |data| bytes.copy_from_slice(&data[offset..offset + 4]));
    if framebuffer.format() == PixelFormat::BGRA8888 {
        (bytes[2], bytes[1], bytes[0], bytes[3])
    } else {
        (bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

fn read_pixel(bitmap: &dyn IReadableBitmapImpl, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let framebuffer = bitmap.lock();
    let pixel = read_framebuffer_pixel(&*framebuffer, x, y);
    framebuffer.dispose();
    pixel
}

/// A 100x100 render target bitmap and helpers to draw into it and read it.
struct Target {
    bitmap: std::sync::Arc<dyn IRenderTargetBitmapImpl>,
}

impl Target {
    fn new() -> Self {
        Self::with_size(100, 100)
    }

    fn with_size(width: i32, height: i32) -> Self {
        Self { bitmap: render_interface().create_render_target_bitmap(PixelSize::new(width, height), DPI) }
    }

    fn draw(&self, f: impl FnOnce(&mut dyn IDrawingContextImpl)) {
        let mut context = self.bitmap.create_drawing_context();
        f(&mut *context);
        context.dispose();
    }

    /// Draws with the backend's own context type.
    fn draw_impl(&self, f: impl FnOnce(&mut DrawingContextImpl)) {
        let mut context = self.bitmap.create_drawing_context();
        f(context.as_any_mut().downcast_mut::<DrawingContextImpl>().expect("a Vello drawing context"));
        context.dispose();
    }

    fn pixel(&self, x: i32, y: i32) -> (u8, u8, u8, u8) {
        read_pixel(&*self.bitmap, x, y)
    }
}

/// The pixels a scene sink renders: premultiplied RGBA.
struct Rendered {
    width: usize,
    rgba: Vec<u8>,
}

impl Rendered {
    fn pixel(&self, x: usize, y: usize) -> (u8, u8, u8, u8) {
        let p = &self.rgba[(y * self.width + x) * 4..][..4];
        (p[0], p[1], p[2], p[3])
    }

    /// The largest difference of a channel of a pixel to another rendering.
    fn largest_difference(&self, other: &Rendered) -> u8 {
        assert_eq!(self.rgba.len(), other.rgba.len());
        self.rgba.iter().zip(&other.rgba).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
    }
}

fn render_sink(sink: &mut dyn IVelloSceneSink) -> Rendered {
    let (width, height) = (sink.width() as usize, sink.height() as usize);
    let mut rgba = vec![0u8; width * height * 4];
    sink.render_to_pixels(&mut rgba);
    Rendered { width, rgba }
}

/// Draws with a drawing context over a scene sink of the caller and
/// returns what the sink renders.
fn draw_into(sink: Box<dyn IVelloSceneSink>, f: impl FnOnce(&mut DrawingContextImpl)) -> Rendered {
    let width = sink.width() as usize;
    let rendered = Rc::new(RefCell::new(Vec::new()));
    let output = rendered.clone();

    let mut context = DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| *output.borrow_mut() = render_sink(sink).rgba),
            scale_drawing_to_dpi: false,
            dpi: DPI,
            rendering_modes: vec![VelloRenderingMode::Cpu],
        },
        Vec::new(),
    );
    f(&mut context);
    context.dispose();

    let rgba = rendered.take();
    Rendered { width, rgba }
}

/// A scene sink of a rendering mode without filters: the scene of the CPU
/// mode with the capabilities and the members of the filters left as the
/// interface has them for a sink that does not say otherwise.
struct SinkWithoutFilters {
    inner: VelloCpuSceneSink,
    /// What was asked of the sink, for the tests to see.
    fills_with_an_image: Rc<RefCell<usize>>,
}

impl SinkWithoutFilters {
    fn new(width: u16, height: u16) -> (Box<dyn IVelloSceneSink>, Rc<RefCell<usize>>) {
        let fills_with_an_image = Rc::new(RefCell::new(0));
        let sink = Self { inner: VelloCpuSceneSink::new(width, height), fills_with_an_image: fills_with_an_image.clone() };
        (Box::new(sink), fills_with_an_image)
    }
}

impl IVelloSceneSink for SinkWithoutFilters {
    fn rendering_mode(&self) -> VelloRenderingMode {
        self.inner.rendering_mode()
    }
    fn capabilities(&self) -> VelloSceneCapabilities {
        self.inner.capabilities()
    }
    fn width(&self) -> u16 {
        self.inner.width()
    }
    fn height(&self) -> u16 {
        self.inner.height()
    }
    fn reset(&mut self) {
        self.inner.reset();
    }
    fn fill(
        &mut self,
        path: &BezPath,
        fill_rule: Fill,
        transform: Affine,
        paint: &VelloScenePaint,
        blend_mode: BlendMode,
        anti_alias: bool,
    ) {
        if matches!(paint.brush, crate::scene::VelloSceneBrush::Image(_)) {
            *self.fills_with_an_image.borrow_mut() += 1;
        }
        self.inner.fill(path, fill_rule, transform, paint, blend_mode, anti_alias);
    }
    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, anti_alias: bool) {
        self.inner.stroke(path, stroke, transform, paint, anti_alias);
    }
    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool) {
        self.inner.push_clip(path, fill_rule, transform, anti_alias);
    }
    fn pop_clip(&mut self) {
        self.inner.pop_clip();
    }
    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32) {
        self.inner.push_layer(blend_mode, opacity);
    }
    fn pop_layer(&mut self) {
        self.inner.pop_layer();
    }
    fn render_to_pixels(&mut self, pixels: &mut [u8]) {
        self.inner.render_to_pixels(pixels);
    }
}

fn cpu_sink(width: u16, height: u16) -> Box<dyn IVelloSceneSink> {
    Box::new(VelloCpuSceneSink::new(width, height))
}

// ---------------------------------------------------------------------------
// Box shadows
// ---------------------------------------------------------------------------

mod box_shadows {
    use super::*;
    use crate::vello_extensions::rounded_rect_path;
    use ferroui_base::Matrix;
    use peniko::color::AlphaColor;

    fn shadow(offset_x: f64, offset_y: f64, blur: f64, spread: f64, color: Color, is_inset: bool) -> BoxShadows {
        BoxShadows::new(BoxShadow { offset_x, offset_y, blur, spread, color, is_inset })
    }

    #[test]
    fn box_shadow_is_drawn_outside_the_box() {
        let target = Target::new();
        let shadows = shadow(0.0, 0.0, 10.0, 5.0, Colors::BLACK, false);
        target.draw(|context| {
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(30.0, 30.0, 40.0, 40.0), &shadows);
        });

        // The box itself.
        assert_eq!(RED, target.pixel(50, 50));
        // The spread right next to the box is nearly opaque black.
        let (r, g, b, a) = target.pixel(27, 50);
        assert!(a > 150 && r == 0 && g == 0 && b == 0, "shadow next to the box: {:?}", target.pixel(27, 50));
        // The shadow fades out.
        let far = target.pixel(18, 50).3;
        assert!(far > 0 && far < a, "the shadow fades: {far} < {a}");
        assert_eq!(TRANSPARENT, target.pixel(2, 50));
    }

    #[test]
    fn box_shadow_offset_moves_the_shadow() {
        let target = Target::new();
        let shadows = shadow(20.0, 0.0, 0.0, 0.0, Colors::BLUE, false);
        target.draw(|context| {
            context.push_render_options(aliased());
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(20.0, 20.0, 40.0, 40.0), &shadows);
            context.pop_render_options();
        });

        assert_eq!(RED, target.pixel(40, 40));
        assert_eq!(BLUE, target.pixel(70, 40));
        assert_eq!(TRANSPARENT, target.pixel(10, 40));
        assert_eq!(TRANSPARENT, target.pixel(85, 40));
    }

    #[test]
    fn inset_box_shadow_is_drawn_inside_the_box() {
        let target = Target::new();
        let shadows = shadow(0.0, 0.0, 0.0, 10.0, Colors::BLUE, true);
        target.draw(|context| {
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(20.0, 20.0, 60.0, 60.0), &shadows);
        });

        // The shadow covers a 10 pixel band inside the box.
        assert_eq!(BLUE, target.pixel(25, 50));
        assert_eq!(BLUE, target.pixel(50, 74));
        // The middle of the box keeps the fill.
        assert_eq!(RED, target.pixel(50, 50));
        // Nothing is drawn outside the box.
        assert_eq!(TRANSPARENT, target.pixel(15, 50));
    }

    // What follows is not from the Skia backend's tests.

    #[test]
    fn a_shadow_is_not_drawn_under_the_box() {
        // A box without a fill: the shadow is everywhere but in the box.
        let target = Target::new();
        let shadows = shadow(0.0, 0.0, 10.0, 5.0, Colors::BLACK, false);
        target.draw(|context| {
            context.draw_rectangle(None, None, rect(30.0, 30.0, 40.0, 40.0), &shadows);
        });

        assert_eq!(TRANSPARENT, target.pixel(50, 50));
        assert_eq!(TRANSPARENT, target.pixel(31, 31));
        assert!(target.pixel(28, 50).3 > 150);
    }

    #[test]
    fn a_blurred_inset_shadow_fades_towards_the_middle() {
        let target = Target::new();
        let shadows = shadow(0.0, 0.0, 12.0, 0.0, Colors::BLUE, true);
        target.draw(|context| {
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(20.0, 20.0, 60.0, 60.0), &shadows);
        });

        // At the edge of the box the shadow is half there, in the middle
        // it is gone.
        let (r, _, b, a) = target.pixel(20, 50);
        assert!(a == 255 && (b as i32 - 127).abs() < 24 && (r as i32 - 128).abs() < 24, "{:?}", target.pixel(20, 50));
        let (r, _, b, _) = target.pixel(26, 50);
        assert!(b > 0 && b < 100 && r > 150, "{:?}", target.pixel(26, 50));
        assert_eq!(RED, target.pixel(50, 50));
        assert_eq!(TRANSPARENT, target.pixel(15, 50));
    }

    #[test]
    fn the_offset_of_a_shadow_is_not_transformed() {
        // The transform of the context is followed by the offset, as in
        // the original: a shadow of a box drawn at twice the size is moved
        // by the offset, not by twice the offset.
        let target = Target::new();
        let shadows = shadow(20.0, 0.0, 0.0, 0.0, Colors::BLUE, false);
        target.draw(|context| {
            context.set_transform(Matrix::create_scale(2.0, 2.0));
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(10.0, 10.0, 20.0, 20.0), &shadows);
        });

        assert_eq!(RED, target.pixel(40, 40));
        assert_eq!(BLUE, target.pixel(70, 40));
        assert_eq!(TRANSPARENT, target.pixel(90, 40));
    }

    #[test]
    fn the_opacity_of_the_context_fades_a_shadow() {
        let target = Target::new();
        let shadows = shadow(30.0, 0.0, 0.0, 0.0, Colors::BLACK, false);
        target.draw(|context| {
            context.push_opacity(0.5, None);
            context.draw_rectangle(None, None, rect(10.0, 10.0, 30.0, 30.0), &shadows);
            context.pop_opacity();
        });

        assert!((target.pixel(55, 25).3 as i32 - 127).abs() <= 1, "{:?}", target.pixel(55, 25));
    }

    #[test]
    fn rounded_rectangles_grow_and_shrink_as_those_of_skia() {
        use crate::drawing_context_impl::box_shadows::outset_rounded_rect;

        let rounded = RoundedRect::new(
            Rect::new(10.0, 10.0, 80.0, 60.0),
            Vector::new(10.0, 5.0),
            Vector::new(0.0, 0.0),
            Vector::new(20.0, 20.0),
            Vector::new(3.0, 3.0),
        );

        // A round corner grows with the rectangle, a square one stays
        // square.
        let grown = outset_rounded_rect(&rounded, 4.0).expect("a rectangle");
        assert_eq!(Rect::new(6.0, 6.0, 88.0, 68.0), grown.rect);
        assert_eq!(Vector::new(14.0, 9.0), grown.radii_top_left);
        assert_eq!(Vector::new(0.0, 0.0), grown.radii_top_right);
        assert_eq!(Vector::new(24.0, 24.0), grown.radii_bottom_right);
        assert_eq!(Vector::new(7.0, 7.0), grown.radii_bottom_left);

        // A corner whose radius is used up is square.
        let shrunk = outset_rounded_rect(&rounded, -5.0).expect("a rectangle");
        assert_eq!(Rect::new(15.0, 15.0, 70.0, 50.0), shrunk.rect);
        assert_eq!(Vector::new(0.0, 0.0), shrunk.radii_top_left);
        assert_eq!(Vector::new(15.0, 15.0), shrunk.radii_bottom_right);
        assert_eq!(Vector::new(0.0, 0.0), shrunk.radii_bottom_left);

        // Nothing is left of a rectangle that is shrunk by half its height.
        assert!(outset_rounded_rect(&rounded, -30.0).is_none());
    }

    /// The Gaussian blur of a rounded rectangle: the blur filter of the
    /// renderer, which the probe of the design document found within 4 of
    /// 255 of the exact blur of an edge.
    fn gaussian(shape: RoundedRect, std_deviation: f64) -> Rendered {
        let mut sink = VelloCpuSceneSink::new(200, 200);
        sink.push_filter_layer(&VelloSceneFilter::Blur { std_deviation: std_deviation as f32 }, Affine::IDENTITY);
        sink.fill(
            &rounded_rect_path(shape),
            Fill::NonZero,
            Affine::IDENTITY,
            &VelloScenePaint::solid(AlphaColor::BLACK),
            BlendMode::default(),
            true,
        );
        sink.pop_layer();
        render_sink(&mut sink)
    }

    fn closed_form(shape: RoundedRect, radius: f64, std_deviation: f64) -> Rendered {
        let mut sink = VelloCpuSceneSink::new(200, 200);
        let r = shape.rect;
        sink.fill_blurred_rounded_rect(
            kurbo::Rect::new(r.x, r.y, r.x + r.width, r.y + r.height),
            radius,
            std_deviation,
            false,
            Affine::IDENTITY,
            AlphaColor::BLACK,
        );
        render_sink(&mut sink)
    }

    #[test]
    fn the_closed_form_of_a_blurred_rounded_rectangle_agrees_with_the_gaussian() {
        use crate::drawing_context_impl::box_shadows::closed_form_radius;

        let (mut drawn_in_closed_form, mut largest) = (0, 0u8);

        for (width, height) in [(80.0, 80.0), (120.0, 30.0), (40.0, 100.0)] {
            for radius in [0.0, 4.0, 7.5, 10.0, 15.0, 20.0] {
                for std_deviation in [0.8, 1.0, 1.37, 2.0, 3.39, 6.27, 12.0] {
                    let shape = RoundedRect::from_radius(Rect::new(40.0, 50.0, width, height), radius);
                    let Some(closed_form_radius) = closed_form_radius(&shape, std_deviation) else {
                        continue;
                    };
                    assert_eq!(radius, closed_form_radius);

                    let difference =
                        gaussian(shape, std_deviation).largest_difference(&closed_form(shape, radius, std_deviation));
                    assert!(
                        difference <= 8,
                        "{width}x{height}, radius {radius}, deviation {std_deviation}: {difference} of 255"
                    );
                    drawn_in_closed_form += 1;
                    largest = largest.max(difference);
                }
            }
        }

        // The rule is not empty: most of the shapes pass it.
        assert!(drawn_in_closed_form >= 40, "{drawn_in_closed_form}");
        println!("{drawn_in_closed_form} shapes in closed form, at most {largest} of 255 from the Gaussian");

        // What the rule keeps out is further off: a capsule, a shape that
        // is thin against its blur, corners that differ.
        let capsule = RoundedRect::from_radius(Rect::new(40.0, 50.0, 120.0, 30.0), 15.0);
        assert!(closed_form_radius(&capsule, 6.27).is_none());
        assert!(gaussian(capsule, 6.27).largest_difference(&closed_form(capsule, 15.0, 6.27)) > 12);
        assert!(closed_form_radius(&RoundedRect::from_rect(Rect::new(0.0, 0.0, 120.0, 30.0)), 12.0).is_none());
        let elliptical = RoundedRect::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Vector::new(10.0, 5.0),
            Vector::new(10.0, 5.0),
            Vector::new(10.0, 5.0),
            Vector::new(10.0, 5.0),
        );
        assert!(closed_form_radius(&elliptical, 2.0).is_none());
    }

    fn draw_shadow(sink: Box<dyn IVelloSceneSink>, shape: RoundedRect, shadows: &BoxShadows) -> Rendered {
        draw_into(sink, |context| context.draw_rectangle(None, None, shape, shadows))
    }

    #[test]
    fn a_shadow_is_the_same_in_closed_form_and_as_an_image() {
        // The same shadows by a sink with blurred rounded rectangles and by
        // one without, which is given the picture of the blurred shape.
        for (shape, shadows) in [
            (rect(30.0, 30.0, 40.0, 40.0), shadow(0.0, 0.0, 10.0, 5.0, Colors::BLACK, false)),
            (
                RoundedRect::from_radius(Rect::new(25.0, 30.0, 50.0, 40.0), 8.0),
                shadow(4.0, 6.0, 8.0, 0.0, Colors::BLACK, false),
            ),
        ] {
            let closed_form = draw_shadow(cpu_sink(100, 100), shape, &shadows);
            let (sink, images) = SinkWithoutFilters::new(100, 100);
            let image = draw_shadow(sink, shape, &shadows);

            assert_eq!(1, *images.borrow(), "the shadow is given to the sink as an image");
            let difference = closed_form.largest_difference(&image);
            assert!(difference <= 10, "{difference} of 255");
        }
    }

    #[test]
    fn a_shadow_of_elliptical_corners_is_blurred_as_an_image() {
        // Corners that are not circular: the shape itself is blurred. The
        // shadow follows the corners: on the diagonal of a corner there is
        // less of it than beside a straight edge.
        let shape = RoundedRect::new(
            Rect::new(20.0, 30.0, 60.0, 40.0),
            Vector::new(30.0, 20.0),
            Vector::new(30.0, 20.0),
            Vector::new(30.0, 20.0),
            Vector::new(30.0, 20.0),
        );
        let shadows = shadow(0.0, 0.0, 6.0, 4.0, Colors::BLACK, false);
        let (sink, images) = SinkWithoutFilters::new(100, 100);
        let rendered = draw_shadow(sink, shape, &shadows);
        assert_eq!(1, *images.borrow());

        // Above the middle of the top edge: inside the spread.
        assert!(rendered.pixel(50, 28).3 > 200, "{:?}", rendered.pixel(50, 28));
        // The corner of the bounds is far from the ellipse.
        assert_eq!(TRANSPARENT, rendered.pixel(19, 29));
        // Nothing in the box.
        assert_eq!(TRANSPARENT, rendered.pixel(50, 50));
        // The shadow is symmetric.
        assert_eq!(rendered.pixel(30, 30), rendered.pixel(69, 30));
        assert_eq!(rendered.pixel(30, 30), rendered.pixel(30, 69));
    }

    #[test]
    fn a_shadow_is_cut_by_the_clips_of_the_context() {
        for sink in [cpu_sink(100, 100), SinkWithoutFilters::new(100, 100).0] {
            let rendered = draw_into(sink, |context| {
                context.push_clip(Rect::new(0.0, 0.0, 100.0, 50.0));
                context.draw_rectangle(None, None, rect(30.0, 30.0, 40.0, 40.0), &shadow(0.0, 0.0, 10.0, 5.0, Colors::BLACK, false));
                context.pop_clip();
            });

            assert!(rendered.pixel(27, 45).3 > 150);
            assert_eq!(TRANSPARENT, rendered.pixel(27, 55));
        }
    }

    #[test]
    fn a_shadow_that_reaches_beyond_the_target_is_not_cut_short() {
        // A box at the edge of the target: its shadow is as dark beside the
        // box as that of a box in the middle.
        let shadows = shadow(0.0, 0.0, 10.0, 5.0, Colors::BLACK, false);
        for sink in [cpu_sink(100, 100), SinkWithoutFilters::new(100, 100).0] {
            let rendered = draw_into(sink, |context| {
                context.draw_rectangle(None, None, rect(-20.0, 30.0, 40.0, 40.0), &shadows);
            });
            let middle = draw_into(cpu_sink(100, 100), |context| {
                context.draw_rectangle(None, None, rect(30.0, 30.0, 40.0, 40.0), &shadows);
            });

            // Three pixels to the right of the box, and above the box at
            // the edge of the target.
            assert!(rendered.pixel(22, 50).3.abs_diff(middle.pixel(72, 50).3) <= 6);
            assert!(rendered.pixel(0, 27).3.abs_diff(middle.pixel(50, 27).3) <= 6, "{:?}", rendered.pixel(0, 27));
        }
    }
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

mod effects {
    use super::*;
    use ferroui_base::media::effects::{ImmutableBlurEffect, ImmutableDropShadowEffect};
    use ferroui_base::platform::IDrawingContextImplWithEffects;
    use ferroui_base::Matrix;

    fn red_square(context: &mut DrawingContextImpl, x: f64) {
        context.push_render_options(aliased());
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(x, 40.0, 20.0, 20.0), &no_shadows());
        context.pop_render_options();
    }

    #[test]
    fn blur_effect_spreads_into_neighbouring_pixels() {
        let plain = Target::new();
        plain.draw_impl(|context| red_square(context, 40.0));
        assert_eq!(TRANSPARENT, plain.pixel(36, 50));
        assert_eq!(RED, plain.pixel(41, 50));

        let blurred = Target::new();
        blurred.draw_impl(|context| {
            context.push_effect(None, &ImmutableBlurEffect::new(10.0));
            red_square(context, 40.0);
            context.pop_effect();
            // After the pop drawing is sharp again.
            context.push_render_options(aliased());
            context.draw_rectangle(Some(&solid(Colors::BLUE)), None, rect(0.0, 0.0, 5.0, 5.0), &no_shadows());
            context.pop_render_options();
        });

        let outside = blurred.pixel(36, 50).3;
        assert!(outside > 20 && outside < 200, "the blur reaches outside the square: {outside}");
        let edge = blurred.pixel(41, 50).3;
        assert!(edge > outside && edge < 255, "the edge is softened: {edge}");
        assert_eq!(TRANSPARENT, blurred.pixel(15, 50));
        assert_eq!(BLUE, blurred.pixel(2, 2));
        assert_eq!(TRANSPARENT, blurred.pixel(6, 2));
    }

    #[test]
    fn blur_effect_without_radius_draws_unchanged() {
        let target = Target::new();
        target.draw_impl(|context| {
            context.push_effect(Some(Rect::new(0.0, 0.0, 100.0, 100.0)), &ImmutableBlurEffect::new(0.0));
            red_square(context, 40.0);
            context.pop_effect();
        });

        assert_eq!(RED, target.pixel(40, 50));
        assert_eq!(TRANSPARENT, target.pixel(39, 50));
    }

    #[test]
    fn effect_with_clip_rect_draws_through_a_bounded_layer() {
        let target = Target::new();
        target.draw_impl(|context| {
            context.push_effect(Some(Rect::new(0.0, 0.0, 50.0, 100.0)), &ImmutableBlurEffect::new(10.0));
            red_square(context, 40.0);
            context.pop_effect();
        });

        // The clip rect sizes the layer of the effect; the blurred content
        // inside it is drawn.
        let inside = target.pixel(45, 50).3;
        assert!(inside > 100, "blurred content inside the clip rect: {inside}");
        let edge = target.pixel(37, 50).3;
        assert!(edge > 0 && edge < inside, "the blur spreads: {edge}");
    }

    #[test]
    fn drop_shadow_effect_is_offset_and_coloured() {
        let target = Target::new();
        target.draw_impl(|context| {
            context.push_effect(None, &ImmutableDropShadowEffect::new(30.0, 10.0, 0.0, Colors::BLUE, 1.0));
            red_square(context, 10.0);
            context.pop_effect();
        });

        // The content itself.
        assert_eq!(RED, target.pixel(20, 50));
        // The shadow: the square moved by (30, 10), in the shadow colour.
        assert_eq!(BLUE, target.pixel(50, 60));
        assert_eq!(BLUE, target.pixel(41, 51));
        assert_eq!(BLUE, target.pixel(58, 68));
        assert_eq!(TRANSPARENT, target.pixel(50, 45));
        assert_eq!(TRANSPARENT, target.pixel(65, 60));
        assert_eq!(TRANSPARENT, target.pixel(35, 50));
    }

    #[test]
    fn drop_shadow_opacity_and_blur_are_applied() {
        let target = Target::new();
        target.draw_impl(|context| {
            context.push_effect(None, &ImmutableDropShadowEffect::new(40.0, 0.0, 0.0, Colors::BLACK, 0.5));
            red_square(context, 10.0);
            context.pop_effect();
        });
        let (r, g, b, a) = target.pixel(60, 50);
        assert!((a as i32 - 127).abs() <= 1 && r == 0 && g == 0 && b == 0, "half opaque black: {:?}", (r, g, b, a));

        // The pushed opacity multiplies into the shadow.
        let faded = Target::new();
        faded.draw_impl(|context| {
            context.push_opacity(0.5, None);
            context.push_effect(None, &ImmutableDropShadowEffect::new(40.0, 0.0, 0.0, Colors::BLACK, 1.0));
            red_square(context, 10.0);
            context.pop_effect();
            context.pop_opacity();
        });
        // The shadow takes its coverage from the (already faded) content.
        assert!((faded.pixel(60, 50).3 as i32 - 63).abs() <= 1, "{:?}", faded.pixel(60, 50));
        assert!((faded.pixel(20, 50).3 as i32 - 127).abs() <= 1);

        let blurred = Target::new();
        blurred.draw_impl(|context| {
            context.push_effect(None, &ImmutableDropShadowEffect::new(40.0, 0.0, 10.0, Colors::BLACK, 1.0));
            red_square(context, 10.0);
            context.pop_effect();
        });
        let outside = blurred.pixel(47, 50).3;
        assert!(outside > 10 && outside < 200, "the shadow is blurred: {outside}");
    }

    // What follows is not from the Skia backend's tests.

    #[test]
    fn the_contract_gives_the_context_with_effects() {
        let target = Target::new();
        target.draw(|context| {
            let effects = context.as_drawing_context_impl_with_effects().expect("a context with effects");
            effects.push_effect(None, &ImmutableBlurEffect::new(10.0));
            effects.draw_rectangle(Some(&solid(Colors::RED)), None, rect(40.0, 40.0, 20.0, 20.0), &no_shadows());
            effects.pop_effect();
        });

        assert!(target.pixel(36, 50).3 > 20);
    }

    /// A blurred square, a square with a blurred shadow and a blur inside
    /// a transform, with sharp shapes before and after.
    fn scene_of_effects(context: &mut DrawingContextImpl) {
        context.draw_rectangle(Some(&solid(Colors::GREEN)), None, rect(0.0, 0.0, 10.0, 10.0), &no_shadows());

        context.push_effect(Some(Rect::new(4.0, 24.0, 52.0, 52.0)), &ImmutableBlurEffect::new(10.0));
        red_square(context, 20.0);
        context.pop_effect();

        context.push_effect(
            Some(Rect::new(60.0, 10.0, 40.0, 40.0)),
            &ImmutableDropShadowEffect::new(6.0, 4.0, 6.0, Colors::BLUE, 0.8),
        );
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(66.0, 16.0, 18.0, 16.0), &no_shadows());
        context.pop_effect();

        let transform = context.transform();
        context.set_transform(Matrix::create_scale(0.5, 0.5) * Matrix::create_translation(50.0, 60.0));
        context.push_effect(None, &ImmutableBlurEffect::new(8.0));
        context.draw_ellipse(Some(&solid(Colors::BLUE)), None, Rect::new(20.0, 10.0, 60.0, 40.0));
        context.pop_effect();
        context.set_transform(transform);

        context.draw_rectangle(Some(&solid(Colors::GREEN)), None, rect(90.0, 90.0, 10.0, 10.0), &no_shadows());
    }

    #[test]
    fn a_rendering_mode_without_filters_draws_an_effect_as_an_image() {
        // The scene by the CPU mode, which has filter layers, and by a sink
        // that has none: what is inside an effect is drawn on the CPU into
        // an image, and the sink is given the image.
        let with_filters = draw_into(cpu_sink(100, 100), scene_of_effects);
        let (sink, images) = SinkWithoutFilters::new(100, 100);
        let without_filters = draw_into(sink, scene_of_effects);

        assert_eq!(3, *images.borrow(), "an image for each effect");
        assert!(with_filters.pixel(36, 50).3 > 20, "the scene has a blur");
        // The same renderer blurs in both: the picture of a layer composed
        // as an image is the layer.
        let difference = with_filters.largest_difference(&without_filters);
        assert!(difference <= 1, "{difference} of 255");
    }

    #[test]
    fn a_sink_without_filters_fails_when_it_is_asked_for_one() {
        let (mut sink, _) = SinkWithoutFilters::new(10, 10);
        assert_eq!(VelloSceneFilterCapabilities::default(), sink.filter_capabilities());
        assert!(!sink.filter_capabilities().filter_layers && !sink.filter_capabilities().blurred_rounded_rects);

        let pushed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sink.push_filter_layer(&VelloSceneFilter::Blur { std_deviation: 1.0 }, Affine::IDENTITY);
        }));
        assert!(pushed.is_err());

        let cpu = VelloCpuSceneSink::new(10, 10);
        assert_eq!(
            VelloSceneFilterCapabilities { filter_layers: true, blurred_rounded_rects: true },
            cpu.filter_capabilities()
        );
    }

    #[test]
    fn an_effect_under_a_clip_is_cut_by_the_clip_and_its_content_is_not() {
        // The blur of a square that a clip cuts in half: outside the clip
        // there is nothing, and inside it the blur is that of the whole
        // square, as on the canvas of Skia.
        let whole = draw_into(cpu_sink(100, 100), |context| {
            context.push_effect(None, &ImmutableBlurEffect::new(10.0));
            red_square(context, 40.0);
            context.pop_effect();
        });
        let clipped = draw_into(cpu_sink(100, 100), |context| {
            context.push_clip(Rect::new(0.0, 0.0, 50.0, 100.0));
            context.push_effect(None, &ImmutableBlurEffect::new(10.0));
            red_square(context, 40.0);
            context.pop_effect();
            context.pop_clip();
        });

        assert!(whole.pixel(52, 50).3 > 200);
        assert_eq!(TRANSPARENT, clipped.pixel(50, 50));
        assert_eq!(TRANSPARENT, clipped.pixel(52, 50));
        for x in 30..50 {
            assert!(whole.pixel(x, 50).3.abs_diff(clipped.pixel(x, 50).3) <= 1, "at {x}");
        }
    }

    #[test]
    fn states_pushed_inside_an_effect_end_inside_it() {
        // Clips, layers, opacity and a mask inside an effect that is
        // recorded into a scene of its own, itself under a clip and inside
        // another effect.
        let rendered = draw_into(cpu_sink(100, 100), |context| {
            context.push_clip(Rect::new(0.0, 0.0, 90.0, 100.0));
            context.push_effect(None, &ImmutableDropShadowEffect::new(0.0, 30.0, 0.0, Colors::BLUE, 1.0));
            context.push_effect(Some(Rect::new(10.0, 10.0, 80.0, 40.0)), &ImmutableBlurEffect::new(2.0));
            context.push_clip(Rect::new(20.0, 20.0, 80.0, 20.0));
            context.push_opacity(0.5, None);
            context.push_layer(Rect::new(0.0, 0.0, 100.0, 100.0));
            context.set_transform(Matrix::create_translation(10.0, 0.0));
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
            context.pop_layer();
            context.pop_opacity();
            context.pop_clip();
            context.pop_effect();
            context.pop_effect();
            context.pop_clip();
            // The transform is that of the push of each state.
            assert_eq!(Matrix::IDENTITY, context.transform());
        });

        // The half opaque red band of the inner clip, blurred a little.
        assert!((rendered.pixel(50, 30).3 as i32 - 127).abs() <= 2, "{:?}", rendered.pixel(50, 30));
        assert_eq!(TRANSPARENT, rendered.pixel(50, 10));
        // Cut by the clip rectangle of the blur at 90 and by the clip.
        assert_eq!(TRANSPARENT, rendered.pixel(95, 30));
        // Its shadow, 30 below.
        let (r, _, b, a) = rendered.pixel(50, 60);
        assert!(r == 0 && b > 100 && (a as i32 - 127).abs() <= 2, "{:?}", rendered.pixel(50, 60));
    }

    #[test]
    fn an_effect_that_is_left_pushed_ends_with_the_context() {
        let rendered = draw_into(SinkWithoutFilters::new(100, 100).0, |context| {
            context.push_effect(Some(Rect::new(20.0, 20.0, 60.0, 60.0)), &ImmutableBlurEffect::new(10.0));
            red_square(context, 40.0);
        });

        assert!(rendered.pixel(36, 50).3 > 20);
    }

    #[test]
    fn an_effect_is_recorded_into_a_scene_as_large_as_its_clip_rectangle() {
        // Content at the corner of the target, blurred into the target
        // from outside of the scene of the effect.
        let draw = |context: &mut DrawingContextImpl| {
            context.push_effect(Some(Rect::new(60.0, 60.0, 60.0, 60.0)), &ImmutableBlurEffect::new(10.0));
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(80.0, 80.0, 40.0, 40.0), &no_shadows());
            context.pop_effect();
        };
        let in_the_scene = draw_into(cpu_sink(100, 100), draw);
        let as_an_image = draw_into(SinkWithoutFilters::new(100, 100).0, draw);

        assert!(in_the_scene.pixel(99, 99).3 > 250, "{:?}", in_the_scene.pixel(99, 99));
        assert!(in_the_scene.pixel(77, 90).3 > 20);
        assert!(in_the_scene.largest_difference(&as_an_image) <= 1);
    }
}
