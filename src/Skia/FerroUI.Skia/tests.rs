//! Render tests: draw through the platform contracts onto raster targets and
//! check the pixels.

use crate::helpers::drawing_context_helper::wrap_skia_surface;
use crate::*;
use ferroui_base::media::imaging::{
    BitmapBlendingMode, BitmapEncoderOptions, BitmapInterpolationMode, CompressionLevel,
    PngBitmapEncoderOptions,
};
use ferroui_base::media::immutable::{
    ImmutableConicGradientBrush, ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush,
    ImmutablePen, ImmutableRadialGradientBrush, ImmutableSolidColorBrush,
};
use ferroui_base::media::{
    BoxShadow, BoxShadows, Color, Colors, EdgeMode, FillRule, GeometryCombineMode, GradientSpreadMethod,
    IBrush, IPen, IntersectionResult, PenLineCap, PenLineJoin, RenderOptions, SweepDirection,
};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget,
    IPlatformRenderSurface,
};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IGeometryImpl, ILockedFramebuffer, IPlatformRenderInterface,
    IReadableBitmapImpl, IRenderTarget, IRenderTargetBitmapImpl, LtrbPixelRect,
    LtrbRect, PixelFormat, RenderTargetSceneInfo,
};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{
    CornerRadius, FerroLocator, Matrix, PixelSize, Point, Rect, RelativePoint, RelativeUnit,
    RoundedRect, Size, Vector,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

const DPI: Vector = Vector::new(96.0, 96.0);

fn render_interface() -> PlatformRenderInterface {
    PlatformRenderInterface::default()
}

fn solid(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn pen(color: Color, thickness: f64) -> ImmutablePen {
    ImmutablePen::with_brush(Some(Rc::new(solid(color))), thickness)
}

fn aliased() -> RenderOptions {
    RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() }
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

    /// The pixel at (x, y) as non-premultiplied-agnostic (r, g, b, a) bytes.
    fn pixel(&self, x: i32, y: i32) -> (u8, u8, u8, u8) {
        read_pixel(&*self.bitmap, x, y)
    }
}

fn read_pixel(bitmap: &dyn IReadableBitmapImpl, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let framebuffer = bitmap.lock();
    let pixel = read_framebuffer_pixel(&*framebuffer, x, y);
    framebuffer.dispose();
    pixel
}

fn read_framebuffer_pixel(framebuffer: &dyn ILockedFramebuffer, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let size = framebuffer.size();
    assert!(x >= 0 && y >= 0 && x < size.width && y < size.height);
    let offset = (y * framebuffer.row_bytes() + x * 4) as usize;
    // SAFETY: the offset is inside the `row_bytes * height` bytes of the
    // locked framebuffer.
    let bytes = unsafe { std::slice::from_raw_parts(framebuffer.address().add(offset), 4) };
    if framebuffer.format() == PixelFormat::BGRA8888 {
        (bytes[2], bytes[1], bytes[0], bytes[3])
    } else {
        (bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);
const BLUE: (u8, u8, u8, u8) = (0, 0, 255, 255);
const TRANSPARENT: (u8, u8, u8, u8) = (0, 0, 0, 0);

fn rect(x: f64, y: f64, w: f64, h: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, w, h))
}

fn png() -> BitmapEncoderOptions {
    PngBitmapEncoderOptions::DEFAULT.into()
}

fn no_shadows() -> BoxShadows {
    BoxShadows::default()
}

// ---------------------------------------------------------------------------
// Drawing context
// ---------------------------------------------------------------------------

#[test]
fn solid_fill_covers_the_rectangle_only() {
    let target = Target::new();
    target.draw(|context| {
        context.push_render_options(aliased());
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(10.0, 10.0, 50.0, 50.0), &no_shadows());
        context.pop_render_options();
    });

    assert_eq!(RED, target.pixel(10, 10));
    assert_eq!(RED, target.pixel(59, 59));
    assert_eq!(TRANSPARENT, target.pixel(9, 9));
    assert_eq!(TRANSPARENT, target.pixel(60, 60));
}

#[test]
fn clear_fills_the_whole_target() {
    let target = Target::new();
    target.draw(|context| context.clear(Colors::BLUE));

    assert_eq!(BLUE, target.pixel(0, 0));
    assert_eq!(BLUE, target.pixel(99, 99));
}

#[test]
fn solid_brush_opacity_scales_alpha() {
    let target = Target::new();
    target.draw(|context| {
        let brush = ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.5);
        context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
    });

    let (r, g, b, a) = target.pixel(50, 50);
    assert_eq!(127, a);
    // Premultiplied.
    assert_eq!((127, 0, 0), (r, g, b));
}

fn linear_brush(start: Color, end: Color, spread: GradientSpreadMethod, end_point: RelativePoint) -> ImmutableLinearGradientBrush {
    ImmutableLinearGradientBrush::new(
        &[ImmutableGradientStop::new(0.0, start), ImmutableGradientStop::new(1.0, end)],
        1.0,
        None,
        None,
        spread,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(end_point),
        None,
    )
}

#[test]
fn linear_gradient_has_its_stop_colors_at_the_endpoints() {
    let target = Target::new();
    target.draw(|context| {
        let brush = linear_brush(
            Colors::RED,
            Colors::BLUE,
            GradientSpreadMethod::Pad,
            RelativePoint::new(1.0, 0.0, RelativeUnit::Relative),
        );
        context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
    });

    let (r, _, b, a) = target.pixel(0, 50);
    assert!(r > 245 && b < 10 && a == 255, "left is red: {:?}", target.pixel(0, 50));
    let (r, _, b, a) = target.pixel(99, 50);
    assert!(b > 245 && r < 10 && a == 255, "right is blue: {:?}", target.pixel(99, 50));
    let (r, _, b, _) = target.pixel(50, 50);
    assert!((r as i32 - b as i32).abs() < 12, "the middle is a mix: {:?}", target.pixel(50, 50));
    // Vertical position does not matter for a horizontal gradient.
    assert_eq!(target.pixel(30, 10), target.pixel(30, 90));
}

#[test]
fn linear_gradient_spread_methods_extend_the_gradient() {
    // The gradient covers the left 25 pixels (absolute end point).
    let end_point = RelativePoint::new(25.0, 0.0, RelativeUnit::Absolute);

    let render = |spread| {
        let target = Target::new();
        target.draw(|context| {
            let brush = linear_brush(Colors::RED, Colors::BLUE, spread, end_point);
            context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
        });
        target
    };

    // Pad: everything past the end has the end color.
    let pad = render(GradientSpreadMethod::Pad);
    assert_eq!(BLUE, pad.pixel(60, 50));
    assert_eq!(BLUE, pad.pixel(99, 50));

    // Repeat: the gradient starts over at 25, 50, 75.
    let repeat = render(GradientSpreadMethod::Repeat);
    let (r, _, b, _) = repeat.pixel(26, 50);
    assert!(r > 220 && b < 35, "repeat restarts with red: {:?}", repeat.pixel(26, 50));
    let (r, _, b, _) = repeat.pixel(48, 50);
    assert!(b > 220 && r < 35, "repeat ends with blue: {:?}", repeat.pixel(48, 50));

    // Reflect: the gradient runs backwards from 25 to 50.
    let reflect = render(GradientSpreadMethod::Reflect);
    let (r, _, b, _) = reflect.pixel(26, 50);
    assert!(b > 220 && r < 35, "reflect continues with blue: {:?}", reflect.pixel(26, 50));
    let (r, _, b, _) = reflect.pixel(48, 50);
    assert!(r > 220 && b < 35, "reflect returns to red: {:?}", reflect.pixel(48, 50));
}

#[test]
fn radial_gradient_goes_from_the_center_to_the_edge() {
    let target = Target::new();
    target.draw(|context| {
        let brush = ImmutableRadialGradientBrush::from_stops(&[
            ImmutableGradientStop::new(0.0, Colors::RED),
            ImmutableGradientStop::new(1.0, Colors::BLUE),
        ]);
        context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
    });

    let (r, _, b, _) = target.pixel(50, 50);
    assert!(r > 240 && b < 15, "the center is red: {:?}", target.pixel(50, 50));
    let (r, _, b, _) = target.pixel(99, 50);
    assert!(b > 240 && r < 15, "the edge is blue: {:?}", target.pixel(99, 50));
    // Beyond the radius the gradient is padded with the last color.
    assert_eq!(BLUE, target.pixel(1, 1));
}

#[test]
fn radial_gradient_with_offset_origin_fills_with_the_final_color() {
    let target = Target::new();
    target.draw(|context| {
        let brush = ImmutableRadialGradientBrush::new(
            &[ImmutableGradientStop::new(0.0, Colors::RED), ImmutableGradientStop::new(1.0, Colors::BLUE)],
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            None,
            Some(RelativePoint::new(0.3, 0.5, RelativeUnit::Relative)),
            None,
            None,
            None,
        );
        context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
    });

    // The focal point has the first color.
    let (r, _, b, _) = target.pixel(30, 50);
    assert!(r > 230 && b < 25, "the origin is red: {:?}", target.pixel(30, 50));
    // Outside the end circle the final color fills the area.
    assert_eq!(BLUE, target.pixel(1, 1));
    assert_eq!(BLUE, target.pixel(98, 98));
}

#[test]
fn conic_gradient_starts_above_the_center() {
    let target = Target::new();
    target.draw(|context| {
        let brush = ImmutableConicGradientBrush::from_stops(&[
            ImmutableGradientStop::new(0.0, Colors::RED),
            ImmutableGradientStop::new(1.0, Colors::BLUE),
        ]);
        context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
    });

    // Angle 0 points up; the sweep runs clockwise. Just right of the upward
    // ray the gradient starts (red), just left of it it ends (blue).
    let (r, _, b, _) = target.pixel(53, 10);
    assert!(r > 230 && b < 25, "start of the sweep is red: {:?}", target.pixel(53, 10));
    let (r, _, b, _) = target.pixel(46, 10);
    assert!(b > 230 && r < 25, "end of the sweep is blue: {:?}", target.pixel(46, 10));
    // Half way round (pointing down) is the mix.
    let (r, _, b, _) = target.pixel(50, 90);
    assert!((r as i32 - b as i32).abs() < 16, "half way is a mix: {:?}", target.pixel(50, 90));
}

#[test]
fn gradient_brush_transform_moves_the_gradient() {
    use ferroui_base::media::immutable::ImmutableTransform;

    let target = Target::new();
    target.draw(|context| {
        let brush = ImmutableLinearGradientBrush::new(
            &[ImmutableGradientStop::new(0.0, Colors::RED), ImmutableGradientStop::new(1.0, Colors::BLUE)],
            1.0,
            Some(Rc::new(ImmutableTransform::new(Matrix::create_translation(50.0, 0.0)))),
            None,
            GradientSpreadMethod::Pad,
            Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
            Some(RelativePoint::new(50.0, 0.0, RelativeUnit::Absolute)),
            None,
        );
        context.draw_rectangle(Some(&brush), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
    });

    // The gradient now spans x = 50..100 instead of 0..50.
    assert_eq!(RED, target.pixel(40, 50));
    let (r, _, b, _) = target.pixel(75, 50);
    assert!((r as i32 - b as i32).abs() < 16, "the middle moved to x = 75: {:?}", target.pixel(75, 50));
}

#[test]
fn clip_limits_drawing() {
    let target = Target::new();
    target.draw(|context| {
        context.push_render_options(aliased());
        context.push_clip(Rect::new(20.0, 20.0, 30.0, 30.0));
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
        context.pop_clip();
        // After popping the clip drawing is unrestricted again.
        context.draw_rectangle(Some(&solid(Colors::BLUE)), None, rect(80.0, 80.0, 10.0, 10.0), &no_shadows());
        context.pop_render_options();
    });

    assert_eq!(RED, target.pixel(20, 20));
    assert_eq!(RED, target.pixel(49, 49));
    assert_eq!(TRANSPARENT, target.pixel(19, 19));
    assert_eq!(TRANSPARENT, target.pixel(50, 50));
    assert_eq!(BLUE, target.pixel(85, 85));
}

#[test]
fn rounded_clip_cuts_the_corners() {
    let target = Target::new();
    target.draw(|context| {
        context.push_clip_rounded(RoundedRect::from_radius(Rect::new(0.0, 0.0, 100.0, 100.0), 30.0));
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
        context.pop_clip();
    });

    assert_eq!(TRANSPARENT, target.pixel(2, 2));
    assert_eq!(TRANSPARENT, target.pixel(97, 97));
    assert_eq!(RED, target.pixel(50, 50));
    assert_eq!(RED, target.pixel(50, 2));
}

#[test]
fn geometry_clip_limits_drawing_to_the_fill() {
    let target = Target::new();
    let clip = render_interface().create_ellipse_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.draw(|context| {
        context.push_geometry_clip(&*clip);
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
        context.pop_geometry_clip();
    });

    assert_eq!(TRANSPARENT, target.pixel(3, 3));
    assert_eq!(RED, target.pixel(50, 50));
}

#[test]
fn region_clip_and_region_fill() {
    let region = render_interface().create_region();
    region.add_rect(LtrbPixelRect { left: 10, top: 10, right: 20, bottom: 20 });
    region.add_rect(LtrbPixelRect { left: 40, top: 40, right: 60, bottom: 60 });

    let clipped = Target::new();
    clipped.draw(|context| {
        context.push_clip_region(&*region);
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
        context.pop_clip();
    });
    assert_eq!(RED, clipped.pixel(15, 15));
    assert_eq!(RED, clipped.pixel(50, 50));
    assert_eq!(TRANSPARENT, clipped.pixel(30, 30));

    let filled = Target::new();
    filled.draw(|context| context.draw_region(Some(&solid(Colors::BLUE)), None, &*region));
    assert_eq!(BLUE, filled.pixel(15, 15));
    assert_eq!(BLUE, filled.pixel(50, 50));
    assert_eq!(TRANSPARENT, filled.pixel(30, 30));
}

#[test]
fn region_tracks_its_rectangles() {
    let region = SkiaRegionImpl::new();
    use ferroui_base::platform::IPlatformRenderInterfaceRegion;

    assert!(region.is_empty());
    region.add_rect(LtrbPixelRect { left: 0, top: 0, right: 10, bottom: 10 });
    region.add_rect(LtrbPixelRect { left: 20, top: 20, right: 30, bottom: 30 });

    assert!(!region.is_empty());
    assert_eq!(LtrbPixelRect { left: 0, top: 0, right: 30, bottom: 30 }, region.bounds());
    assert_eq!(2, region.rects().len());
    assert!(region.contains(Point::new(5.0, 5.0)));
    assert!(!region.contains(Point::new(15.0, 15.0)));
    assert!(region.intersects(LtrbRect { left: 25.5, top: 25.5, right: 40.0, bottom: 40.0 }));
    assert!(!region.intersects(LtrbRect { left: 11.0, top: 11.0, right: 19.0, bottom: 19.0 }));

    region.reset();
    assert!(region.is_empty());
    assert!(region.rects().is_empty());
}

#[test]
fn pushed_opacity_multiplies_alpha() {
    let target = Target::new();
    target.draw(|context| {
        context.push_opacity(0.5, None);
        context.push_opacity(0.5, None);
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 50.0, 100.0), &no_shadows());
        context.pop_opacity();
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(50.0, 0.0, 50.0, 100.0), &no_shadows());
        context.pop_opacity();
    });

    assert_eq!(63, target.pixel(25, 50).3);
    assert_eq!(127, target.pixel(75, 50).3);
}

#[test]
fn full_opacity_handling_composites_overlapping_shapes_as_one() {
    let draw = |requires_full_opacity_handling| {
        let target = Target::new();
        target.draw(|context| {
            context.push_render_options(RenderOptions {
                requires_full_opacity_handling: Some(requires_full_opacity_handling),
                ..RenderOptions::default()
            });
            context.push_opacity(0.5, Some(Rect::new(0.0, 0.0, 100.0, 100.0)));
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 60.0, 100.0), &no_shadows());
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(40.0, 0.0, 60.0, 100.0), &no_shadows());
            context.pop_opacity();
            context.pop_render_options();
        });
        target
    };

    // Per-primitive opacity: the overlap is drawn twice and gets denser.
    let flat = draw(false);
    assert!(flat.pixel(50, 50).3 > 180);
    assert_eq!(127, flat.pixel(10, 50).3);

    // Layer opacity: the group is composited once.
    let layered = draw(true);
    assert!((layered.pixel(50, 50).3 as i32 - 127).abs() <= 1);
    assert!((layered.pixel(10, 50).3 as i32 - 127).abs() <= 1);
}

#[test]
fn opacity_mask_modulates_alpha() {
    let target = Target::new();
    target.draw(|context| {
        // Opaque on the left, transparent on the right.
        let mask = linear_brush(
            Colors::BLACK,
            Colors::TRANSPARENT,
            GradientSpreadMethod::Pad,
            RelativePoint::new(1.0, 0.0, RelativeUnit::Relative),
        );
        context.push_opacity_mask(&mask, Rect::new(0.0, 0.0, 100.0, 100.0));
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 100.0, 100.0), &no_shadows());
        context.pop_opacity_mask();
    });

    assert!(target.pixel(1, 50).3 > 245, "left is opaque: {:?}", target.pixel(1, 50));
    assert!(target.pixel(98, 50).3 < 10, "right is transparent: {:?}", target.pixel(98, 50));
    let middle = target.pixel(50, 50).3 as i32;
    assert!((middle - 127).abs() < 10, "the middle is half transparent: {middle}");
}

#[test]
fn transform_moves_and_scales_drawing() {
    let target = Target::new();
    target.draw(|context| {
        context.push_render_options(aliased());
        assert_eq!(Matrix::IDENTITY, context.transform());

        let transform = Matrix::create_scale(2.0, 2.0) * Matrix::create_translation(30.0, 40.0);
        context.set_transform(transform);
        assert_eq!(transform, context.transform());

        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 10.0, 10.0), &no_shadows());
        context.pop_render_options();
    });

    assert_eq!(RED, target.pixel(30, 40));
    assert_eq!(RED, target.pixel(49, 59));
    assert_eq!(TRANSPARENT, target.pixel(29, 39));
    assert_eq!(TRANSPARENT, target.pixel(50, 60));
}

#[test]
fn transform_is_restored_with_the_canvas_state() {
    let target = Target::new();
    target.draw(|context| {
        let transform = Matrix::create_translation(10.0, 10.0);
        context.set_transform(transform);
        context.push_clip(Rect::new(0.0, 0.0, 50.0, 50.0));
        context.set_transform(Matrix::create_translation(70.0, 70.0));
        context.pop_clip();

        // Popping the clip restores the canvas, including its matrix.
        assert_eq!(transform, context.transform());
    });
}

#[test]
fn stroke_has_the_pen_thickness() {
    let target = Target::new();
    target.draw(|context| {
        context.push_render_options(aliased());
        context.draw_line(Some(&pen(Colors::RED, 6.0)), Point::new(0.0, 50.0), Point::new(100.0, 50.0));
        context.pop_render_options();
    });

    let covered = (0..100).filter(|y| target.pixel(50, *y) == RED).count();
    assert_eq!(6, covered);
    assert_eq!(RED, target.pixel(50, 47));
    assert_eq!(RED, target.pixel(50, 52));
    assert_eq!(TRANSPARENT, target.pixel(50, 46));
    assert_eq!(TRANSPARENT, target.pixel(50, 53));
}

#[test]
fn rectangle_stroke_is_centered_on_the_outline() {
    let target = Target::new();
    target.draw(|context| {
        context.push_render_options(aliased());
        context.draw_rectangle(None, Some(&pen(Colors::BLUE, 4.0)), rect(20.0, 20.0, 60.0, 60.0), &no_shadows());
        context.pop_render_options();
    });

    assert_eq!(BLUE, target.pixel(18, 50));
    assert_eq!(BLUE, target.pixel(21, 50));
    assert_eq!(TRANSPARENT, target.pixel(17, 50));
    assert_eq!(TRANSPARENT, target.pixel(22, 50));
    assert_eq!(TRANSPARENT, target.pixel(50, 50));
}

#[test]
fn pen_line_caps_extend_the_line() {
    let draw = |cap| {
        let target = Target::new();
        let pen = ImmutablePen::new(Some(Rc::new(solid(Colors::RED))), 10.0, None, cap, PenLineJoin::Miter, 10.0);
        target.draw(|context| {
            context.push_render_options(aliased());
            context.draw_line(Some(&pen), Point::new(30.0, 50.0), Point::new(70.0, 50.0));
            context.pop_render_options();
        });
        target
    };

    let flat = draw(PenLineCap::Flat);
    assert_eq!(RED, flat.pixel(30, 50));
    assert_eq!(TRANSPARENT, flat.pixel(28, 50));

    let square = draw(PenLineCap::Square);
    assert_eq!(RED, square.pixel(26, 50));
    assert_eq!(RED, square.pixel(25, 46));
    assert_eq!(TRANSPARENT, square.pixel(24, 50));

    let round = draw(PenLineCap::Round);
    assert_eq!(RED, round.pixel(26, 50));
    // The corner of the square cap is outside the round one.
    assert_eq!(TRANSPARENT, round.pixel(25, 46));
}

#[test]
fn dashed_pen_leaves_gaps() {
    let target = Target::new();
    let pen = ImmutablePen::new(
        Some(Rc::new(solid(Colors::RED))),
        2.0,
        Some(Rc::new(ImmutableDashStyle::new(Some(&[5.0, 5.0]), 0.0))),
        PenLineCap::Flat,
        PenLineJoin::Miter,
        10.0,
    );
    target.draw(|context| {
        context.push_render_options(aliased());
        context.draw_line(Some(&pen), Point::new(0.0, 50.0), Point::new(100.0, 50.0));
        context.pop_render_options();
    });

    // Dashes are relative to the thickness: 10 on, 10 off.
    assert_eq!(RED, target.pixel(5, 50));
    assert_eq!(TRANSPARENT, target.pixel(15, 50));
    assert_eq!(RED, target.pixel(25, 50));
    assert_eq!(TRANSPARENT, target.pixel(35, 50));
}

#[test]
fn rounded_rectangle_has_rounded_corners() {
    let target = Target::new();
    target.draw(|context| {
        context.draw_rectangle(
            Some(&solid(Colors::RED)),
            None,
            RoundedRect::from_radius(Rect::new(0.0, 0.0, 100.0, 100.0), 25.0),
            &no_shadows(),
        );
    });

    for (x, y) in [(2, 2), (97, 2), (2, 97), (97, 97)] {
        assert_eq!(TRANSPARENT, target.pixel(x, y), "corner ({x}, {y})");
    }
    assert_eq!(RED, target.pixel(50, 1));
    assert_eq!(RED, target.pixel(1, 50));
    assert_eq!(RED, target.pixel(50, 50));
}

#[test]
fn rounded_rectangle_supports_individual_corner_radii() {
    assert!(render_interface().supports_individual_round_rects());

    let target = Target::new();
    target.draw(|context| {
        context.draw_rectangle(
            Some(&solid(Colors::RED)),
            None,
            RoundedRect::from_corner_radius(Rect::new(0.0, 0.0, 100.0, 100.0), CornerRadius::new(40.0, 0.0, 0.0, 0.0)),
            &no_shadows(),
        );
    });

    // Only the top left corner is rounded.
    assert_eq!(TRANSPARENT, target.pixel(3, 3));
    assert_eq!(RED, target.pixel(96, 3));
    assert_eq!(RED, target.pixel(3, 96));
    assert_eq!(RED, target.pixel(96, 96));
}

#[test]
fn box_shadow_is_drawn_outside_the_box() {
    let target = Target::new();
    let shadows = BoxShadows::new(BoxShadow {
        offset_x: 0.0,
        offset_y: 0.0,
        blur: 10.0,
        spread: 5.0,
        color: Colors::BLACK,
        is_inset: false,
    });
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
    let shadows = BoxShadows::new(BoxShadow {
        offset_x: 20.0,
        offset_y: 0.0,
        blur: 0.0,
        spread: 0.0,
        color: Colors::BLUE,
        is_inset: false,
    });
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
    let shadows = BoxShadows::new(BoxShadow {
        offset_x: 0.0,
        offset_y: 0.0,
        blur: 0.0,
        spread: 10.0,
        color: Colors::BLUE,
        is_inset: true,
    });
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

#[test]
fn ellipse_is_filled_and_stroked() {
    let target = Target::new();
    target.draw(|context| {
        context.draw_ellipse(Some(&solid(Colors::RED)), Some(&pen(Colors::BLUE, 4.0)), Rect::new(10.0, 10.0, 80.0, 80.0));
    });

    assert_eq!(RED, target.pixel(50, 50));
    assert_eq!(BLUE, target.pixel(50, 10));
    assert_eq!(TRANSPARENT, target.pixel(12, 12));
}

#[test]
fn geometry_is_filled_and_stroked() {
    let render_interface = render_interface();
    let geometry = render_interface.create_stream_geometry();
    {
        let mut context = geometry.open();
        context.begin_figure(Point::new(20.0, 20.0), true);
        context.line_to(Point::new(80.0, 20.0), true);
        context.line_to(Point::new(80.0, 80.0), true);
        context.line_to(Point::new(20.0, 80.0), true);
        context.end_figure(true);
        context.dispose();
    }

    let target = Target::new();
    target.draw(|context| {
        context.push_render_options(aliased());
        context.draw_geometry(Some(&solid(Colors::RED)), Some(&pen(Colors::BLUE, 2.0)), &*geometry);
        context.pop_render_options();
    });

    assert_eq!(RED, target.pixel(50, 50));
    assert_eq!(BLUE, target.pixel(20, 50));
    assert_eq!(BLUE, target.pixel(50, 79));
    assert_eq!(TRANSPARENT, target.pixel(17, 50));
}

#[test]
fn layer_can_be_drawn_into_and_blitted() {
    let target = Target::new();
    target.draw(|context| {
        let layer = context.create_layer(PixelSize::new(100, 100));
        assert!(layer.can_blit());
        assert!(!layer.is_corrupted());
        assert_eq!(1, layer.version());

        let mut layer_context = layer.create_drawing_context();
        layer_context.push_render_options(aliased());
        layer_context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(10.0, 10.0, 20.0, 20.0), &no_shadows());
        layer_context.pop_render_options();
        layer_context.dispose();
        assert_eq!(2, layer.version());

        // The blit ignores the transform of the target context.
        context.set_transform(Matrix::create_translation(50.0, 50.0));
        layer.blit(context);
        assert_eq!(Matrix::create_translation(50.0, 50.0), context.transform());

        // Layers are bitmaps as well and can be drawn scaled.
        context.set_transform(Matrix::IDENTITY);
        context.draw_bitmap(&*layer, 1.0, Rect::new(10.0, 10.0, 20.0, 20.0), Rect::new(60.0, 60.0, 30.0, 30.0));
        layer.dispose();
    });

    assert_eq!(RED, target.pixel(15, 15));
    assert_eq!(TRANSPARENT, target.pixel(40, 40));
    assert_eq!(RED, target.pixel(75, 75));
}

#[test]
fn push_layer_isolates_drawing_until_popped() {
    let target = Target::new();
    target.draw(|context| {
        context.clear(Colors::BLUE);
        context.push_layer(Rect::new(0.0, 0.0, 100.0, 100.0));
        context.push_render_options(RenderOptions {
            bitmap_blending_mode: BitmapBlendingMode::Source,
            ..RenderOptions::default()
        });
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 50.0, 100.0), &no_shadows());
        context.pop_render_options();
        context.pop_layer();
    });

    assert_eq!(RED, target.pixel(25, 50));
    assert_eq!(BLUE, target.pixel(75, 50));
}

fn checker_bitmap() -> std::sync::Arc<ferroui_base::platform::SharedBitmapImpl> {
    // 2x2: red, blue / blue, red (BGRA premultiplied).
    let red = [0u8, 0, 255, 255];
    let blue = [255u8, 0, 0, 255];
    let mut data = Vec::new();
    data.extend_from_slice(&red);
    data.extend_from_slice(&blue);
    data.extend_from_slice(&blue);
    data.extend_from_slice(&red);

    render_interface().load_bitmap_from_pixels(
        PixelFormat::BGRA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(2, 2),
        DPI,
        8,
    )
}

#[test]
fn bitmap_is_drawn_with_interpolation_mode() {
    let bitmap = checker_bitmap();
    let source = Rect::new(0.0, 0.0, 2.0, 2.0);
    let dest = Rect::new(0.0, 0.0, 100.0, 100.0);

    // Nearest neighbour keeps the hard edge between the texels.
    let nearest = Target::new();
    nearest.draw(|context| {
        context.push_render_options(RenderOptions {
            bitmap_interpolation_mode: BitmapInterpolationMode::None,
            ..RenderOptions::default()
        });
        context.draw_bitmap(&*bitmap, 1.0, source, dest);
        context.pop_render_options();
    });
    assert_eq!(RED, nearest.pixel(25, 25));
    assert_eq!(BLUE, nearest.pixel(75, 25));
    assert_eq!(RED, nearest.pixel(48, 25));
    assert_eq!(BLUE, nearest.pixel(51, 25));

    // Linear filtering blends across the edge.
    let linear = Target::new();
    linear.draw(|context| {
        context.push_render_options(RenderOptions {
            bitmap_interpolation_mode: BitmapInterpolationMode::LowQuality,
            ..RenderOptions::default()
        });
        context.draw_bitmap(&*bitmap, 1.0, source, dest);
        context.pop_render_options();
    });
    let (r, _, b, _) = linear.pixel(50, 25);
    assert!(r > 80 && b > 80, "the edge is blended: {:?}", linear.pixel(50, 25));
}

#[test]
fn bitmap_opacity_and_blend_mode_are_applied() {
    let bitmap = checker_bitmap();
    let source = Rect::new(0.0, 0.0, 1.0, 1.0);
    let dest = Rect::new(0.0, 0.0, 100.0, 100.0);

    let faded = Target::new();
    faded.draw(|context| context.draw_bitmap(&*bitmap, 0.5, source, dest));
    assert_eq!(127, faded.pixel(50, 50).3);

    // Destination blending keeps what is already there.
    let kept = Target::new();
    kept.draw(|context| {
        context.clear(Colors::BLUE);
        context.push_render_options(RenderOptions {
            bitmap_blending_mode: BitmapBlendingMode::Destination,
            ..RenderOptions::default()
        });
        context.draw_bitmap(&*bitmap, 1.0, source, dest);
        context.pop_render_options();
    });
    assert_eq!(BLUE, kept.pixel(50, 50));
}

#[test]
fn bitmap_can_be_drawn_through_an_opacity_mask() {
    let bitmap = checker_bitmap();
    let target = Target::new();
    target.draw(|context| {
        let mask = linear_brush(
            Colors::BLACK,
            Colors::TRANSPARENT,
            GradientSpreadMethod::Pad,
            RelativePoint::new(1.0, 0.0, RelativeUnit::Relative),
        );
        context.draw_bitmap_with_mask(&*bitmap, &mask, Rect::new(0.0, 0.0, 100.0, 100.0), Rect::new(0.0, 0.0, 100.0, 100.0));
    });

    assert!(target.pixel(2, 50).3 > 240);
    assert!(target.pixel(97, 50).3 < 15);
}

#[test]
fn render_options_are_merged_and_restored() {
    let bitmap = std::sync::Arc::new(RenderTargetBitmapImpl::new(PixelSize::new(10, 10), DPI));
    let mut context = bitmap.create_drawing_context();

    context.push_render_options(aliased());
    context.push_render_options(RenderOptions {
        bitmap_interpolation_mode: BitmapInterpolationMode::HighQuality,
        ..RenderOptions::default()
    });
    // No way to observe the options through the contract; popping in order
    // must leave the stack balanced.
    context.pop_render_options();
    context.pop_render_options();
    context.dispose();

    // The concrete type exposes them.
    let surface = skia_safe::surfaces::raster_n32_premul((10, 10)).unwrap();
    let mut context = wrap_skia_surface(&surface, DPI);
    context.push_render_options(aliased());
    context.push_render_options(RenderOptions {
        bitmap_interpolation_mode: BitmapInterpolationMode::HighQuality,
        ..RenderOptions::default()
    });
    assert_eq!(EdgeMode::Aliased, context.render_options().edge_mode);
    assert_eq!(BitmapInterpolationMode::HighQuality, context.render_options().bitmap_interpolation_mode);
    context.pop_render_options();
    assert_eq!(BitmapInterpolationMode::Unspecified, context.render_options().bitmap_interpolation_mode);
    context.pop_render_options();
    assert_eq!(RenderOptions::default(), context.render_options());
    context.dispose();
}

#[test]
fn antialiasing_follows_the_edge_mode() {
    let draw = |options: RenderOptions| {
        let target = Target::new();
        target.draw(|context| {
            context.push_render_options(options);
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(10.5, 10.5, 50.0, 50.0), &no_shadows());
            context.pop_render_options();
        });
        target
    };

    // Antialiased (the default): the half covered edge pixel is translucent.
    let smooth = draw(RenderOptions::default());
    let alpha = smooth.pixel(10, 30).3;
    assert!(alpha > 100 && alpha < 155, "antialiased edge: {alpha}");

    // Aliased: pixels are either covered or not.
    let hard = draw(aliased());
    let alpha = hard.pixel(10, 30).3;
    assert!(alpha == 0 || alpha == 255, "aliased edge: {alpha}");
}

#[test]
fn lease_gives_access_to_the_surface_and_blocks_the_context() {
    let bitmap = std::sync::Arc::new(RenderTargetBitmapImpl::new(PixelSize::new(20, 20), DPI));
    let mut context = bitmap.create_drawing_context();
    context.set_transform(Matrix::create_translation(5.0, 5.0));

    let feature = context
        .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
        .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned())
        .expect("surface backed contexts provide the lease feature");

    let lease = feature.lease();
    assert!(lease.sk_surface().is_some());
    assert!(lease.gr_context().is_none());
    assert!(lease.try_lease_platform_graphics_api().is_none());
    assert_eq!(1.0, lease.current_opacity());
    lease.with_sk_canvas(&mut |canvas| {
        canvas.reset_matrix();
        canvas.clear(skia_safe::Color::RED);
    });

    // While leased the context refuses to draw.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| context.clear(Colors::BLUE)));
    assert!(result.is_err());

    lease.dispose();

    // The lease restored the matrix it found.
    context.push_render_options(aliased());
    context.draw_rectangle(Some(&solid(Colors::BLUE)), None, rect(0.0, 0.0, 5.0, 5.0), &no_shadows());
    context.pop_render_options();
    context.dispose();

    assert_eq!(RED, read_pixel(&*bitmap, 2, 2));
    assert_eq!(BLUE, read_pixel(&*bitmap, 7, 7));
}

#[test]
fn opacity_save_layer_option_is_read_from_the_locator() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind_to_self(Rc::new(SkiaOptions { use_opacity_save_layer: true, ..SkiaOptions::default() }));

    let target = Target::new();
    target.draw(|context| {
        context.push_opacity(0.5, None);
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 60.0, 100.0), &no_shadows());
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(40.0, 0.0, 60.0, 100.0), &no_shadows());
        context.pop_opacity();
    });
    scope.dispose();

    assert!((target.pixel(50, 50).3 as i32 - 127).abs() <= 1);
}

#[test]
fn picture_render_target_records_drawing() {
    let picture_target = PictureRenderTarget::new(None, None, DPI);
    let mut context = picture_target.create_drawing_context(Size::new(50.0, 50.0), true);
    context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 50.0, 50.0), &no_shadows());
    // Recorder backed contexts have no surface to lease.
    assert!(context.get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>()).is_none());
    context.dispose();

    let picture = picture_target.get_picture();
    assert_eq!(skia_safe::Rect::new(0.0, 0.0, 50.0, 50.0), picture.cull_rect());

    let mut surface = skia_safe::surfaces::raster_n32_premul((50, 50)).unwrap();
    surface.canvas().draw_picture(&picture, None, None);
    let mut pixels = [0u8; 4];
    let info = skia_safe::ImageInfo::new(
        (1, 1),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    assert!(surface.read_pixels(&info, &mut pixels, 4, (25, 25)));
    assert_eq!([255, 0, 0, 255], pixels);
}

#[test]
fn glyph_run_is_drawn_with_the_foreground() {
    let font_mgr = skia_safe::FontMgr::new();
    let Some(typeface) = font_mgr.legacy_make_typeface(None, skia_safe::FontStyle::normal()) else {
        // No fonts on this machine; nothing to draw with.
        return;
    };
    let font = skia_safe::Font::from_typeface(typeface, 12.0);

    let mut glyph_ids = [0u16; 2];
    font.str_to_glyphs("HI", &mut glyph_ids);
    let sized = font.with_size(40.0).unwrap();
    let mut widths = [0f32; 2];
    sized.get_widths(&glyph_ids, &mut widths);

    let glyphs: Vec<ferroui_base::media::text_formatting::GlyphInfo> = glyph_ids
        .iter()
        .zip(widths)
        .enumerate()
        .map(|(cluster, (glyph_index, width))| {
            ferroui_base::media::text_formatting::GlyphInfo::new(*glyph_index, cluster as i32, width as f64)
        })
        .collect();

    let glyph_run = GlyphRunImpl::from_font(&font, 40.0, &glyphs, Point::new(10.0, 60.0));

    use ferroui_base::platform::IGlyphRunImpl;
    assert_eq!(40.0, glyph_run.font_rendering_em_size());
    assert_eq!(Point::new(10.0, 60.0), glyph_run.baseline_origin());
    let bounds = glyph_run.bounds();
    assert!(bounds.width > 20.0 && bounds.height > 20.0, "the run has bounds: {bounds}");
    assert!(bounds.x >= 9.0 && bounds.bottom() <= 62.0, "the bounds sit on the baseline: {bounds}");
    // A horizontal band through the letters intersects their stems.
    assert!(!glyph_run.get_intersections(-20.0, -10.0).is_empty());

    let target = Target::new();
    target.draw(|context| {
        context.clear(Colors::WHITE);
        context.draw_glyph_run(Some(&solid(Colors::BLACK)), &glyph_run);
        // No foreground, no drawing.
        context.draw_glyph_run(None, &glyph_run);
    });

    let mut dark = 0;
    for y in 0..100 {
        for x in 0..100 {
            let (r, g, b, _) = target.pixel(x, y);
            if r < 60 && g < 60 && b < 60 {
                dark += 1;
                assert!(
                    bounds.inflate(1.5).contains(Point::new(x as f64, y as f64)),
                    "ink at ({x}, {y}) is inside the run bounds {bounds}"
                );
            }
        }
    }
    assert!(dark > 100, "the glyphs left ink: {dark} dark pixels");

    glyph_run.dispose();
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[test]
fn rectangle_geometry_hit_testing_and_bounds() {
    let geometry = render_interface().create_rectangle_geometry(Rect::new(10.0, 10.0, 80.0, 60.0));

    assert_eq!(Rect::new(10.0, 10.0, 80.0, 60.0), geometry.bounds());
    assert!(geometry.fill_contains(Point::new(50.0, 40.0)));
    assert!(!geometry.fill_contains(Point::new(5.0, 40.0)));

    let pen = pen(Colors::BLACK, 4.0);
    assert!(geometry.stroke_contains(Some(&pen), Point::new(11.0, 40.0)));
    assert!(geometry.stroke_contains(Some(&pen), Point::new(9.0, 40.0)));
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(50.0, 40.0)));
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(5.0, 40.0)));
    assert!(!geometry.stroke_contains(None, Point::new(10.0, 40.0)));

    assert_eq!(Rect::new(8.0, 8.0, 84.0, 64.0), geometry.get_render_bounds(Some(&pen)));
    assert_eq!(Rect::new(10.0, 10.0, 80.0, 60.0), geometry.get_render_bounds(None));

    assert!((geometry.contour_length() - 280.0).abs() < 0.01);
}

#[test]
fn ellipse_geometry_respects_fill_and_stroke() {
    // Mirrors the hit-testing scenarios of an ellipse with a fill and with a
    // stroke only.
    let geometry = render_interface().create_ellipse_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), geometry.bounds());
    assert!(!geometry.fill_contains(Point::new(10.0, 10.0)));
    assert!(geometry.fill_contains(Point::new(50.0, 50.0)));

    let pen = pen(Colors::RED, 5.0);
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(50.0, 50.0)));
    assert!(geometry.stroke_contains(Some(&pen), Point::new(1.0, 50.0)));
}

#[test]
fn line_geometry_has_a_stroke_but_no_fill() {
    let geometry = render_interface().create_line_geometry(Point::new(80.0, 20.0), Point::new(20.0, 60.0));

    assert_eq!(Rect::new(20.0, 20.0, 60.0, 40.0), geometry.bounds());
    assert!(!geometry.fill_contains(Point::new(50.0, 40.0)));
    assert!(geometry.stroke_contains(Some(&pen(Colors::BLACK, 4.0)), Point::new(50.0, 40.0)));
    assert!((geometry.contour_length() - (60.0f64 * 60.0 + 40.0 * 40.0).sqrt()).abs() < 0.01);

    let (point, tangent) = geometry.try_get_point_and_tangent_at_distance(0.0).unwrap();
    assert!(point.nearly_equals(Point::new(80.0, 20.0)));
    assert!(tangent.x < 0.0 && tangent.y > 0.0);
    assert!(geometry.try_get_point_at_distance(1000.0).is_some());
}

#[test]
fn stream_geometry_context_builds_paths() {
    let geometry = StreamGeometryImpl::new();
    assert_eq!(Rect::default(), geometry.bounds());

    use ferroui_base::platform::IStreamGeometryImpl;
    let mut context = geometry.open();
    context.set_fill_rule(FillRule::NonZero);
    context.begin_figure(Point::new(10.0, 10.0), true);
    context.line_to(Point::new(90.0, 10.0), true);
    context.quadratic_bezier_to(Point::new(100.0, 50.0), Point::new(90.0, 90.0), true);
    context.cubic_bezier_to(Point::new(70.0, 100.0), Point::new(30.0, 100.0), Point::new(10.0, 90.0), true);
    context.end_figure(true);
    context.dispose();

    let stroke = geometry.stroke_path().unwrap();
    assert_eq!(skia_safe::PathFillType::Winding, stroke.fill_type());
    assert!(geometry.is_fill_same_as_stroke());
    use skia_safe::PathVerb;
    assert_eq!(
        &[PathVerb::Move, PathVerb::Line, PathVerb::Quad, PathVerb::Cubic, PathVerb::Close],
        stroke.verbs()
    );

    let bounds = geometry.bounds();
    assert!((bounds.x - 10.0).abs() < 0.01 && (bounds.y - 10.0).abs() < 0.01);
    assert!(bounds.right() > 90.0 && bounds.right() < 100.0, "tight bounds exclude the control point: {bounds}");
    assert!(geometry.fill_contains(Point::new(50.0, 50.0)));
    assert!(!geometry.fill_contains(Point::new(5.0, 5.0)));
}

#[test]
fn stream_geometry_arc_is_added_as_an_elliptical_arc() {
    let geometry = StreamGeometryImpl::new();
    use ferroui_base::platform::IStreamGeometryImpl;
    let mut context = geometry.open();
    context.begin_figure(Point::new(0.0, 0.0), true);
    context.arc_to(Point::new(128.0, 0.0), Size::new(128.0, 128.0), 0.0, false, SweepDirection::CounterClockwise, true);
    context.end_figure(false);
    context.dispose();

    // The chord is 128 wide on a circle of radius 128: the sagitta is
    // 128 - sqrt(128^2 - 64^2) = 17.1487...
    let bounds = geometry.bounds();
    assert!((bounds.width - 128.0).abs() < 0.001, "{bounds}");
    assert!((bounds.height - 17.14875).abs() < 0.001, "{bounds}");
}

#[test]
fn stream_geometry_unfilled_figures_are_stroked_only() {
    let geometry = StreamGeometryImpl::new();
    use ferroui_base::platform::IStreamGeometryImpl;
    let mut context = geometry.open();
    // A filled square...
    context.begin_figure(Point::new(0.0, 0.0), true);
    context.line_to(Point::new(40.0, 0.0), true);
    context.line_to(Point::new(40.0, 40.0), true);
    context.line_to(Point::new(0.0, 40.0), true);
    context.end_figure(true);
    // ...and an unfilled one.
    context.begin_figure(Point::new(60.0, 0.0), false);
    context.line_to(Point::new(100.0, 0.0), true);
    context.line_to(Point::new(100.0, 40.0), true);
    context.line_to(Point::new(60.0, 40.0), true);
    context.end_figure(true);
    context.dispose();

    assert!(!geometry.is_fill_same_as_stroke());
    assert!(geometry.fill_contains(Point::new(20.0, 20.0)));
    assert!(!geometry.fill_contains(Point::new(80.0, 20.0)));

    let pen = pen(Colors::BLACK, 2.0);
    assert!(geometry.stroke_contains(Some(&pen), Point::new(0.0, 20.0)));
    assert!(geometry.stroke_contains(Some(&pen), Point::new(100.0, 20.0)));
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 40.0), geometry.bounds());
}

#[test]
fn stream_geometry_unstroked_segments_break_the_stroke() {
    let geometry = StreamGeometryImpl::new();
    use ferroui_base::platform::IStreamGeometryImpl;
    let mut context = geometry.open();
    context.begin_figure(Point::new(0.0, 0.0), true);
    context.line_to(Point::new(50.0, 0.0), true);
    // Not stroked: the fill still gets the edge.
    context.line_to(Point::new(50.0, 50.0), false);
    context.line_to(Point::new(0.0, 50.0), true);
    context.end_figure(true);
    context.dispose();

    let pen = pen(Colors::BLACK, 2.0);
    assert!(geometry.stroke_contains(Some(&pen), Point::new(25.0, 0.0)));
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(50.0, 25.0)));
    assert!(geometry.stroke_contains(Some(&pen), Point::new(25.0, 50.0)));
    // The closing edge is stroked with an explicit line.
    assert!(geometry.stroke_contains(Some(&pen), Point::new(0.0, 25.0)));
    assert!(geometry.fill_contains(Point::new(45.0, 25.0)));
}

#[test]
fn stream_geometry_clone_is_independent() {
    let geometry = StreamGeometryImpl::new();
    use ferroui_base::platform::IStreamGeometryImpl;
    let mut context = geometry.open();
    context.begin_figure(Point::new(0.0, 0.0), true);
    context.line_to(Point::new(10.0, 0.0), true);
    context.line_to(Point::new(10.0, 10.0), true);
    context.end_figure(true);
    context.dispose();

    let clone = geometry.clone_geometry();
    assert_eq!(geometry.bounds(), clone.bounds());

    let mut context = clone.open();
    context.begin_figure(Point::new(50.0, 50.0), true);
    context.line_to(Point::new(60.0, 50.0), true);
    context.line_to(Point::new(60.0, 60.0), true);
    context.end_figure(true);
    context.dispose();

    assert_eq!(Rect::new(0.0, 0.0, 10.0, 10.0), geometry.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 60.0, 60.0), clone.bounds());
    assert!(clone.as_stream_geometry().is_some());
}

#[test]
fn stream_geometry_caches_are_invalidated_when_reopened() {
    let geometry = StreamGeometryImpl::new();
    use ferroui_base::platform::IStreamGeometryImpl;
    let pen = pen(Colors::BLACK, 2.0);

    let mut context = geometry.open();
    context.begin_figure(Point::new(0.0, 0.0), false);
    context.line_to(Point::new(10.0, 0.0), true);
    context.end_figure(false);
    context.dispose();

    assert_eq!(Rect::new(0.0, -1.0, 10.0, 2.0), geometry.get_render_bounds(Some(&pen)));
    assert!((geometry.contour_length() - 10.0).abs() < 0.001);

    let mut context = geometry.open();
    context.line_to(Point::new(30.0, 0.0), true);
    context.dispose();

    assert_eq!(Rect::new(0.0, -1.0, 30.0, 2.0), geometry.get_render_bounds(Some(&pen)));
    assert!((geometry.contour_length() - 30.0).abs() < 0.001);
}

#[test]
fn transformed_geometry_applies_the_transform() {
    let source = render_interface().create_rectangle_geometry(Rect::new(0.0, 0.0, 10.0, 10.0));
    let transform = Matrix::create_scale(2.0, 3.0) * Matrix::create_translation(5.0, 5.0);
    let transformed = source.with_transform(transform);

    assert_eq!(Rect::new(5.0, 5.0, 20.0, 30.0), transformed.bounds());
    assert_eq!(transform, transformed.transform());
    assert_eq!(source.bounds(), transformed.source_geometry().bounds());
    assert!(transformed.fill_contains(Point::new(20.0, 30.0)));
    assert!(!transformed.fill_contains(Point::new(2.0, 2.0)));
    assert!(transformed.as_transformed_geometry().is_some());
    assert!(source.as_transformed_geometry().is_none());

    // A transformed line still has no fill.
    let line = render_interface().create_line_geometry(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    let moved = line.with_transform(Matrix::create_translation(0.0, 5.0));
    assert!(!moved.fill_contains(Point::new(5.0, 5.0)));
    assert!(moved.stroke_contains(Some(&pen(Colors::BLACK, 2.0)), Point::new(5.0, 5.0)));
}

#[test]
fn combined_geometry_modes() {
    let render_interface = render_interface();
    let a = render_interface.create_rectangle_geometry(Rect::new(0.0, 0.0, 60.0, 60.0));
    let b = render_interface.create_rectangle_geometry(Rect::new(40.0, 40.0, 60.0, 60.0));

    let in_a_only = Point::new(10.0, 10.0);
    let in_both = Point::new(50.0, 50.0);
    let in_b_only = Point::new(90.0, 90.0);

    let union = render_interface.create_combined_geometry(GeometryCombineMode::Union, a.clone(), b.clone());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), union.bounds());
    assert!(union.fill_contains(in_a_only) && union.fill_contains(in_both) && union.fill_contains(in_b_only));

    let intersect = render_interface.create_combined_geometry(GeometryCombineMode::Intersect, a.clone(), b.clone());
    assert_eq!(Rect::new(40.0, 40.0, 20.0, 20.0), intersect.bounds());
    assert!(!intersect.fill_contains(in_a_only) && intersect.fill_contains(in_both));

    let xor = render_interface.create_combined_geometry(GeometryCombineMode::Xor, a.clone(), b.clone());
    assert!(xor.fill_contains(in_a_only) && !xor.fill_contains(in_both) && xor.fill_contains(in_b_only));

    let exclude = render_interface.create_combined_geometry(GeometryCombineMode::Exclude, a.clone(), b.clone());
    assert!(exclude.fill_contains(in_a_only) && !exclude.fill_contains(in_both) && !exclude.fill_contains(in_b_only));

    // `intersect` on the geometry itself.
    let intersection = a.intersect(&*b).unwrap();
    assert_eq!(Rect::new(40.0, 40.0, 20.0, 20.0), intersection.bounds());
}

#[test]
fn geometry_group_combines_children_with_the_fill_rule() {
    let render_interface = render_interface();
    let outer = render_interface.create_rectangle_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
    let inner = render_interface.create_rectangle_geometry(Rect::new(25.0, 25.0, 50.0, 50.0));

    let even_odd = render_interface.create_geometry_group(FillRule::EvenOdd, &[outer.clone(), inner.clone()]);
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), even_odd.bounds());
    assert!(even_odd.fill_contains(Point::new(10.0, 10.0)));
    assert!(!even_odd.fill_contains(Point::new(50.0, 50.0)));

    let non_zero = render_interface.create_geometry_group(FillRule::NonZero, &[outer.clone(), inner.clone()]);
    assert!(non_zero.fill_contains(Point::new(50.0, 50.0)));

    // A child without a fill (a line) forces a separate fill pass.
    let line = render_interface.create_line_geometry(Point::new(150.0, 0.0), Point::new(150.0, 100.0));
    let with_line = render_interface.create_geometry_group(FillRule::EvenOdd, &[outer, line]);
    assert_eq!(Rect::new(0.0, 0.0, 150.0, 100.0), with_line.bounds());
    assert!(with_line.fill_contains(Point::new(50.0, 50.0)));
    assert!(with_line.stroke_contains(Some(&pen(Colors::BLACK, 4.0)), Point::new(150.0, 50.0)));
}

#[test]
fn fill_intersection_results() {
    let render_interface = render_interface();
    let big = render_interface.create_rectangle_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
    let small = render_interface.create_rectangle_geometry(Rect::new(20.0, 20.0, 20.0, 20.0));
    let overlapping = render_interface.create_rectangle_geometry(Rect::new(80.0, 80.0, 50.0, 50.0));
    let apart = render_interface.create_rectangle_geometry(Rect::new(200.0, 200.0, 10.0, 10.0));
    let line = render_interface.create_line_geometry(Point::new(0.0, 0.0), Point::new(10.0, 10.0));

    assert_eq!(IntersectionResult::FullyInside, big.get_fill_intersection_result(&*small));
    assert_eq!(IntersectionResult::FullyContains, small.get_fill_intersection_result(&*big));
    assert_eq!(IntersectionResult::Intersects, big.get_fill_intersection_result(&*overlapping));
    assert_eq!(IntersectionResult::Empty, big.get_fill_intersection_result(&*apart));
    assert_eq!(IntersectionResult::Empty, big.get_fill_intersection_result(&*line));
}

#[test]
fn widened_geometry_is_the_outline_of_the_stroke() {
    let line = render_interface().create_line_geometry(Point::new(10.0, 50.0), Point::new(90.0, 50.0));
    let widened = line.get_widened_geometry(&pen(Colors::BLACK, 10.0));

    assert_eq!(Rect::new(10.0, 45.0, 80.0, 10.0), widened.bounds());
    assert!(widened.fill_contains(Point::new(50.0, 48.0)));
    assert!(!widened.fill_contains(Point::new(50.0, 40.0)));

    // A zero thickness pen has no outline.
    let empty = line.get_widened_geometry(&pen(Colors::BLACK, 0.0));
    assert_eq!(Rect::default(), empty.bounds());
    assert!(!empty.fill_contains(Point::new(50.0, 50.0)));
}

#[test]
fn geometry_segment_snips_a_part_of_the_contour() {
    let line = render_interface().create_line_geometry(Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    let segment = line.try_get_segment(20.0, 50.0, true).unwrap();
    assert_eq!(Rect::new(20.0, 0.0, 30.0, 0.0), segment.bounds());
    assert!((segment.contour_length() - 30.0).abs() < 0.001);

    assert!(line.try_get_segment(50.0, 20.0, true).is_none());
}

#[test]
fn geometries_of_another_backend_are_not_recognized() {
    struct Foreign;
    impl IGeometryImpl for Foreign {
        fn bounds(&self) -> Rect {
            Rect::new(0.0, 0.0, 10.0, 10.0)
        }
        fn contour_length(&self) -> f64 {
            0.0
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn get_render_bounds(&self, _: Option<&dyn IPen>) -> Rect {
            Rect::default()
        }
        fn get_widened_geometry(&self, _: &dyn IPen) -> Arc<dyn IGeometryImpl> {
            Arc::new(Foreign)
        }
        fn fill_contains(&self, _: Point) -> bool {
            true
        }
        fn get_fill_intersection_result(&self, _: &dyn IGeometryImpl) -> IntersectionResult {
            IntersectionResult::Empty
        }
        fn intersect(&self, _: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
            None
        }
        fn stroke_contains(&self, _: Option<&dyn IPen>, _: Point) -> bool {
            false
        }
        fn with_transform(&self, _: Matrix) -> Arc<dyn ferroui_base::platform::ITransformedGeometryImpl> {
            unimplemented!()
        }
        fn try_get_point_at_distance(&self, _: f64) -> Option<Point> {
            None
        }
        fn try_get_point_and_tangent_at_distance(&self, _: f64) -> Option<(Point, Point)> {
            None
        }
        fn try_get_segment(&self, _: f64, _: f64, _: bool) -> Option<Arc<dyn IGeometryImpl>> {
            None
        }
    }

    let render_interface = render_interface();
    let ours = render_interface.create_rectangle_geometry(Rect::new(0.0, 0.0, 10.0, 10.0));
    let foreign: Arc<dyn IGeometryImpl> = Arc::new(Foreign);

    assert!(try_get_geometry_impl(&*ours).is_some());
    assert!(try_get_geometry_impl(&*foreign).is_none());
    assert!(ours.intersect(&*foreign).is_none());
    assert_eq!(IntersectionResult::Empty, ours.get_fill_intersection_result(&*foreign));

    let combined = render_interface.create_combined_geometry(GeometryCombineMode::Union, ours.clone(), foreign.clone());
    assert_eq!(Rect::default(), combined.bounds());

    let group = render_interface.create_geometry_group(FillRule::EvenOdd, &[ours.clone(), foreign]);
    assert_eq!(Rect::new(0.0, 0.0, 10.0, 10.0), group.bounds());
}

// ---------------------------------------------------------------------------
// Bitmaps
// ---------------------------------------------------------------------------

fn red_writeable_bitmap(width: i32, height: i32) -> std::sync::Arc<WriteableBitmapImpl> {
    let bitmap = WriteableBitmapImpl::new(PixelSize::new(width, height), DPI, PixelFormat::BGRA8888, AlphaFormat::Premul);
    let framebuffer = bitmap.lock();
    // SAFETY: the locked framebuffer is `row_bytes * height` bytes long.
    let pixels = unsafe {
        std::slice::from_raw_parts_mut(framebuffer.address(), (framebuffer.row_bytes() * height) as usize)
    };
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 255, 255]);
    }
    framebuffer.dispose();
    bitmap
}

#[test]
fn writeable_bitmap_lock_bumps_the_version() {
    let bitmap = WriteableBitmapImpl::new(PixelSize::new(4, 4), DPI, PixelFormat::RGBA8888, AlphaFormat::Premul);
    assert_eq!(1, bitmap.version());
    assert_eq!(Some(PixelFormat::RGBA8888), bitmap.format());
    assert_eq!(Some(AlphaFormat::Premul), bitmap.alpha_format());
    assert_eq!(TRANSPARENT, read_pixel(&*bitmap, 1, 1));
    assert_eq!(2, bitmap.version());

    let framebuffer = bitmap.lock();
    assert_eq!(PixelSize::new(4, 4), framebuffer.size());
    assert_eq!(16, framebuffer.row_bytes());
    assert_eq!(DPI, framebuffer.dpi());
    assert_eq!(PixelFormat::RGBA8888, framebuffer.format());
    framebuffer.dispose();
    // Disposing twice does nothing.
    framebuffer.dispose();
    assert_eq!(3, bitmap.version());
}

#[test]
fn writeable_bitmap_is_redrawn_after_its_pixels_change() {
    let bitmap = red_writeable_bitmap(4, 4);
    let target = Target::new();
    target.draw(|context| {
        context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 4.0, 4.0), Rect::new(0.0, 0.0, 50.0, 50.0));
    });
    assert_eq!(RED, target.pixel(25, 25));

    // Paint it blue and draw again: the cached snapshot must not be reused.
    let framebuffer = bitmap.lock();
    // SAFETY: the locked framebuffer is `row_bytes * height` bytes long.
    let pixels = unsafe { std::slice::from_raw_parts_mut(framebuffer.address(), 64) };
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[255, 0, 0, 255]);
    }
    framebuffer.dispose();

    target.draw(|context| {
        context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 4.0, 4.0), Rect::new(50.0, 50.0, 50.0, 50.0));
    });
    assert_eq!(BLUE, target.pixel(75, 75));
}

#[test]
fn png_compression_levels_all_encode() {
    let bitmap = red_writeable_bitmap(64, 64);
    let mut sizes = Vec::new();
    for compression_level in [
        CompressionLevel::NoCompression,
        CompressionLevel::Fastest,
        CompressionLevel::Optimal,
        CompressionLevel::SmallestSize,
    ] {
        let mut stream = Vec::new();
        bitmap.save(&mut stream, &PngBitmapEncoderOptions { compression_level }.into()).unwrap();
        sizes.push(stream.len());
    }
    // An uncompressed PNG of a flat image is the largest.
    assert!(sizes[0] > sizes[2], "{sizes:?}");
}

#[test]
fn bitmap_round_trips_through_encode_and_decode() {
    let render_interface = render_interface();

    // Draw something recognisable, encode it, decode it and compare.
    let target = Target::with_size(32, 16);
    target.draw(|context| {
        context.push_render_options(aliased());
        context.clear(Colors::WHITE);
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 16.0, 16.0), &no_shadows());
        context.draw_rectangle(Some(&solid(Colors::BLUE)), None, rect(16.0, 8.0, 16.0, 8.0), &no_shadows());
        context.pop_render_options();
    });

    let source = target.bitmap.as_any().downcast_ref::<RenderTargetBitmapImpl>().unwrap();
    let mut encoded = Vec::new();
    IBitmapImpl::save(source, &mut encoded, &png()).unwrap();

    let decoded = render_interface.load_bitmap(&mut encoded.as_slice()).unwrap();
    assert_eq!(PixelSize::new(32, 16), decoded.pixel_size());
    assert_eq!(DPI, decoded.dpi());
    assert_eq!(1, decoded.version());

    let decoded_immutable = decoded.as_any().downcast_ref::<ImmutableBitmap>().unwrap();
    assert!(decoded_immutable.format().is_some());
    for (x, y, expected) in [(4, 4, RED), (20, 12, BLUE), (20, 2, (255, 255, 255, 255))] {
        assert_eq!(expected, read_pixel(decoded_immutable, x, y), "decoded pixel ({x}, {y})");
        assert_eq!(expected, target.pixel(x, y), "source pixel ({x}, {y})");
    }

    // A writeable bitmap decodes the same data.
    let writeable = render_interface.load_writeable_bitmap(&mut encoded.as_slice()).unwrap();
    assert_eq!(RED, read_pixel(&*writeable, 4, 4));

    // Decoding to a width keeps the aspect ratio.
    let half = render_interface
        .load_bitmap_to_width(&mut encoded.as_slice(), 16, BitmapInterpolationMode::HighQuality)
        .unwrap();
    assert_eq!(PixelSize::new(16, 8), half.pixel_size());
    let tall = render_interface
        .load_writeable_bitmap_to_height(&mut encoded.as_slice(), 32, BitmapInterpolationMode::LowQuality)
        .unwrap();
    assert_eq!(PixelSize::new(64, 32), tall.pixel_size());
    assert_eq!(RED, read_pixel(&*tall, 8, 8));

    // Resizing.
    let resized = render_interface.resize_bitmap(&*decoded, PixelSize::new(8, 4), BitmapInterpolationMode::None);
    assert_eq!(PixelSize::new(8, 4), resized.pixel_size());
    assert_eq!(RED, read_pixel(resized.as_any().downcast_ref::<ImmutableBitmap>().unwrap(), 1, 1));

    // Garbage is rejected.
    assert!(render_interface.load_bitmap(&mut [1u8, 2, 3, 4].as_slice()).is_err());
    assert!(render_interface.load_writeable_bitmap(&mut [1u8, 2, 3, 4].as_slice()).is_err());
    assert!(render_interface.load_bitmap_from_file("/nonexistent/file.png").is_err());
}

#[test]
fn render_interface_reports_its_capabilities() {
    let render_interface = render_interface();

    assert_eq!(AlphaFormat::Premul, render_interface.default_alpha_format());
    assert!(render_interface.is_supported_bitmap_pixel_format(render_interface.default_pixel_format()));
    assert!(render_interface.is_supported_bitmap_pixel_format(PixelFormat::RGB565));
    assert!(!render_interface.is_supported_bitmap_pixel_format(PixelFormat::RGB32));
    assert!(render_interface.supports_regions());
    assert!(render_interface.create_region().is_empty());
}

// ---------------------------------------------------------------------------
// Framebuffer render target
// ---------------------------------------------------------------------------

/// A framebuffer in plain memory that records how it is used.
struct MockFramebuffer {
    pixels: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    size: PixelSize,
    row_bytes: i32,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    disposed: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl ILockedFramebuffer for MockFramebuffer {
    fn address(&self) -> *mut u8 {
        self.pixels.lock().unwrap().as_mut_ptr()
    }
    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.pixels.lock().unwrap());
    }
    fn size(&self) -> PixelSize {
        self.size
    }
    fn row_bytes(&self) -> i32 {
        self.row_bytes
    }
    fn dpi(&self) -> Vector {
        DPI
    }
    fn format(&self) -> PixelFormat {
        self.format
    }
    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }
    fn dispose(&self) {
        self.disposed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

struct MockSurface {
    pixels: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    size: PixelSize,
    row_bytes: i32,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    disposed: std::sync::Arc<std::sync::atomic::AtomicU32>,
    retains: bool,
}

impl MockSurface {
    fn new(width: i32, height: i32, format: PixelFormat, alpha_format: AlphaFormat) -> std::sync::Arc<Self> {
        // A stride larger than the minimum, as real framebuffers often have.
        let row_bytes = width * 4 + 8;
        std::sync::Arc::new(Self {
            pixels: std::sync::Arc::new(std::sync::Mutex::new(vec![0u8; (row_bytes * height) as usize])),
            size: PixelSize::new(width, height),
            row_bytes,
            format,
            alpha_format,
            disposed: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            retains: true,
        })
    }

    fn bytes(&self, x: i32, y: i32) -> [u8; 4] {
        let offset = (y * self.row_bytes + x * 4) as usize;
        let pixels = self.pixels.lock().unwrap();
        [pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3]]
    }
}

impl IPlatformRenderSurface for MockSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for MockSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let (pixels, size, row_bytes, format, alpha_format, disposed) =
            (self.pixels.clone(), self.size, self.row_bytes, self.format, self.alpha_format, self.disposed.clone());
        Rc::new(FuncFramebufferRenderTarget::with_scene_info(
            move |_| {
                let framebuffer: Rc<dyn ILockedFramebuffer> = Rc::new(MockFramebuffer {
                    pixels: pixels.clone(),
                    size,
                    row_bytes,
                    format,
                    alpha_format,
                    disposed: disposed.clone(),
                });
                (framebuffer, FramebufferLockProperties { previous_frame_is_retained: true })
            },
            self.retains,
        ))
    }
}

fn scene_info(size: PixelSize) -> RenderTargetSceneInfo {
    RenderTargetSceneInfo::new(size, 1.0, CompositionTransparencyLevel::None)
}

fn draw_red_square(render_target: &dyn IRenderTarget, size: PixelSize) {
    let (mut context, _) = render_target.create_drawing_context(&scene_info(size));
    context.push_render_options(aliased());
    context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(2.0, 2.0, 4.0, 4.0), &no_shadows());
    context.pop_render_options();
    context.dispose();
}

#[test]
fn framebuffer_render_target_writes_bgra() {
    let surface = MockSurface::new(8, 8, PixelFormat::BGRA8888, AlphaFormat::Premul);
    let render_target = FramebufferRenderTarget::new(&*surface, false);

    assert!(render_target.properties().is_suitable_for_direct_rendering);
    assert!(render_target.properties().retains_previous_frame_contents);

    let (mut context, properties) = render_target.create_drawing_context(&scene_info(surface.size));
    assert!(properties.previous_frame_is_retained);
    context.push_render_options(aliased());
    context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(2.0, 2.0, 4.0, 4.0), &no_shadows());
    context.pop_render_options();
    assert_eq!(0, surface.disposed.load(std::sync::atomic::Ordering::SeqCst));
    context.dispose();

    // The framebuffer is presented exactly once, when the context is disposed.
    assert_eq!(1, surface.disposed.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!([0, 0, 255, 255], surface.bytes(3, 3));
    assert_eq!([0, 0, 0, 0], surface.bytes(1, 1));
    assert_eq!([0, 0, 0, 0], surface.bytes(6, 6));

    // The surface over the framebuffer is reused for the next frame.
    let (mut context, _) = render_target.create_drawing_context(&scene_info(surface.size));
    context.push_render_options(aliased());
    context.draw_rectangle(Some(&solid(Colors::BLUE)), None, rect(0.0, 0.0, 2.0, 2.0), &no_shadows());
    context.pop_render_options();
    context.dispose();
    assert_eq!(2, surface.disposed.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!([255, 0, 0, 255], surface.bytes(1, 1));
    // The previous frame is still there.
    assert_eq!([0, 0, 255, 255], surface.bytes(3, 3));

    render_target.dispose();
    assert_eq!(ferroui_base::platform::PlatformRenderTargetState::DISPOSED, render_target.platform_render_target_state());
}

#[test]
fn framebuffer_render_target_writes_rgba() {
    let surface = MockSurface::new(8, 8, PixelFormat::RGBA8888, AlphaFormat::Premul);
    let render_target = FramebufferRenderTarget::new(&*surface, false);

    draw_red_square(&render_target, surface.size);

    assert_eq!([255, 0, 0, 255], surface.bytes(3, 3));
    assert_eq!([0, 0, 0, 0], surface.bytes(1, 1));
    render_target.dispose();
}

#[test]
fn framebuffer_render_target_writes_unpremultiplied_pixels() {
    let surface = MockSurface::new(8, 8, PixelFormat::BGRA8888, AlphaFormat::Unpremul);
    let render_target = FramebufferRenderTarget::new(&*surface, false);

    let (mut context, _) = render_target.create_drawing_context(&scene_info(surface.size));
    let brush = ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.5);
    context.push_render_options(aliased());
    context.draw_rectangle(Some(&brush), None, rect(2.0, 2.0, 4.0, 4.0), &no_shadows());
    context.pop_render_options();
    context.dispose();

    assert_eq!(1, surface.disposed.load(std::sync::atomic::Ordering::SeqCst));
    // Unpremultiplied: full red with half alpha.
    let [b, g, r, a] = surface.bytes(3, 3);
    assert!((a as i32 - 127).abs() <= 1, "half alpha: {:?}", surface.bytes(3, 3));
    assert!(r >= 250 && g == 0 && b == 0, "unpremultiplied red: {:?}", surface.bytes(3, 3));
    assert_eq!(0, surface.bytes(1, 1)[3]);
    render_target.dispose();
}

#[test]
fn framebuffer_render_target_applies_scaling_for_bitmaps() {
    // Render target bitmaps draw in logical units: at 192 DPI everything is
    // twice as large in pixels.
    let bitmap = render_interface().create_render_target_bitmap(PixelSize::new(20, 20), Vector::new(192.0, 192.0));
    let mut context = bitmap.create_drawing_context();
    context.push_render_options(aliased());
    context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 5.0, 5.0), &no_shadows());
    context.pop_render_options();
    context.dispose();

    assert_eq!(RED, read_pixel(&*bitmap, 9, 9));
    assert_eq!(TRANSPARENT, read_pixel(&*bitmap, 10, 10));
}

#[test]
fn software_context_creates_render_targets_for_framebuffer_surfaces() {
    let render_interface = render_interface();
    let context = render_interface.create_backend_context(None);
    assert!(!context.is_lost());
    assert!(context.max_offscreen_render_target_pixel_size().is_none());

    let surface = MockSurface::new(8, 8, PixelFormat::BGRA8888, AlphaFormat::Premul);
    let surfaces: Vec<std::sync::Arc<dyn IPlatformRenderSurface>> = vec![surface.clone()];
    assert!(context.is_ready_to_create_render_target(&surfaces));
    assert!(!context.is_ready_to_create_render_target(&[]));

    let render_target = context.create_render_target(&surfaces);
    draw_red_square(&*render_target, surface.size);
    assert_eq!([0, 0, 255, 255], surface.bytes(3, 3));
    render_target.dispose();

    // Offscreen render targets are layers.
    let offscreen = context.create_offscreen_render_target(PixelSize::new(10, 10), Vector::new(2.0, 2.0), true);
    assert_eq!(PixelSize::new(10, 10), offscreen.pixel_size());
    assert_eq!(Vector::new(192.0, 192.0), offscreen.dpi());
    let mut offscreen_context = offscreen.create_drawing_context();
    offscreen_context.clear(Colors::BLUE);
    offscreen_context.dispose();

    let target = Target::new();
    target.draw(|context| offscreen.blit(context));
    assert_eq!(BLUE, target.pixel(5, 5));
    assert_eq!(TRANSPARENT, target.pixel(15, 15));

    // A surface render target can be saved.
    let layer = offscreen.as_any().downcast_ref::<SurfaceRenderTarget>().unwrap();
    assert!(!layer.has_render_context_affinity());
    let mut encoded = Vec::new();
    layer.save(&mut encoded, &png()).unwrap();
    let decoded = ImmutableBitmap::from_stream(&mut encoded.as_slice()).unwrap();
    assert_eq!(BLUE, read_pixel(&decoded, 5, 5));

    offscreen.dispose();
    context.dispose();
}

#[test]
fn render_target_bitmap_is_a_framebuffer_surface() {
    let bitmap = std::sync::Arc::new(RenderTargetBitmapImpl::new(PixelSize::new(8, 8), DPI));
    assert!(!bitmap.is_corrupted());

    let context = render_interface().create_backend_context(None);
    let surfaces: Vec<std::sync::Arc<dyn IPlatformRenderSurface>> = vec![bitmap.clone()];
    let render_target = context.create_render_target(&surfaces);
    draw_red_square(&*render_target, PixelSize::new(8, 8));

    assert_eq!(RED, read_pixel(&*bitmap, 3, 3));
    render_target.dispose();
    IBitmapImpl::dispose(&*bitmap);
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

mod effects_and_brushes {
    use super::*;
    use crate::DrawingContextImpl;
    use ferroui_base::media::effects::{ImmutableBlurEffect, ImmutableDropShadowEffect};
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::media::immutable::{ImmutableImageBrush, ImmutableTransform};
    use ferroui_base::media::{
        AcrylicBackgroundSource, AlignmentX, AlignmentY, IExperimentalAcrylicMaterial, IImmutableBrush, ISceneBrush,
        ISceneBrushContent, ITileBrush, ITransform, ImmutableSceneBrush, Stretch, TileMode,
    };
    use ferroui_base::platform::{IDrawingContextImplWithEffects, IDrawingContextWithAcrylicLikeSupport};
    use ferroui_base::RelativeRect;

    /// Draws with the backend's own context type, which is what exposes the
    /// effect and acrylic members.
    fn draw(target: &Target, f: impl FnOnce(&mut DrawingContextImpl)) {
        let mut context = target.bitmap.create_drawing_context();
        f(context.as_any_mut().downcast_mut::<DrawingContextImpl>().expect("a Skia drawing context"));
        context.dispose();
    }

    fn red_square(context: &mut DrawingContextImpl, x: f64) {
        context.push_render_options(aliased());
        context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(x, 40.0, 20.0, 20.0), &no_shadows());
        context.pop_render_options();
    }

    #[test]
    fn blur_effect_spreads_into_neighbouring_pixels() {
        let plain = Target::new();
        draw(&plain, |context| red_square(context, 40.0));
        assert_eq!(TRANSPARENT, plain.pixel(36, 50));
        assert_eq!(RED, plain.pixel(41, 50));

        let blurred = Target::new();
        draw(&blurred, |context| {
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
        draw(&target, |context| {
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
        draw(&target, |context| {
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
        draw(&target, |context| {
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
        draw(&target, |context| {
            context.push_effect(None, &ImmutableDropShadowEffect::new(40.0, 0.0, 0.0, Colors::BLACK, 0.5));
            red_square(context, 10.0);
            context.pop_effect();
        });
        let (r, g, b, a) = target.pixel(60, 50);
        assert!((a as i32 - 127).abs() <= 1 && r == 0 && g == 0 && b == 0, "half opaque black: {:?}", (r, g, b, a));

        // The pushed opacity multiplies into the shadow.
        let faded = Target::new();
        draw(&faded, |context| {
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
        draw(&blurred, |context| {
            context.push_effect(None, &ImmutableDropShadowEffect::new(40.0, 0.0, 10.0, Colors::BLACK, 1.0));
            red_square(context, 10.0);
            context.pop_effect();
        });
        let outside = blurred.pixel(47, 50).3;
        assert!(outside > 10 && outside < 200, "the shadow is blurred: {outside}");
    }

    // -----------------------------------------------------------------------
    // Image brushes
    // -----------------------------------------------------------------------

    /// A 20x10 bitmap: the left half red, the right half blue.
    fn two_tone_bitmap() -> Rc<Bitmap> {
        let mut data = Vec::new();
        for _ in 0..10 {
            for x in 0..20 {
                data.extend_from_slice(if x < 10 { &[0u8, 0, 255, 255] } else { &[255u8, 0, 0, 255] });
            }
        }

        Rc::new(Bitmap::from_impl(render_interface().load_bitmap_from_pixels(
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
            &data,
            PixelSize::new(20, 10),
            DPI,
            80,
        )))
    }

    struct BrushSpec {
        alignment_x: AlignmentX,
        alignment_y: AlignmentY,
        destination_rect: Option<RelativeRect>,
        source_rect: Option<RelativeRect>,
        stretch: Stretch,
        tile_mode: TileMode,
        opacity: f64,
        transform: Option<Matrix>,
    }

    impl Default for BrushSpec {
        fn default() -> Self {
            Self {
                alignment_x: AlignmentX::Center,
                alignment_y: AlignmentY::Center,
                destination_rect: None,
                source_rect: None,
                stretch: Stretch::Uniform,
                tile_mode: TileMode::None,
                opacity: 1.0,
                transform: None,
            }
        }
    }

    fn image_brush(spec: BrushSpec) -> ImmutableImageBrush {
        ImmutableImageBrush::new(
            Some(two_tone_bitmap()),
            spec.alignment_x,
            spec.alignment_y,
            spec.destination_rect,
            spec.opacity,
            spec.transform.map(|transform| Rc::new(ImmutableTransform::new(transform))),
            RelativePoint::default(),
            spec.source_rect,
            spec.stretch,
            spec.tile_mode,
            None,
        )
    }

    fn fill_with(brush: &dyn IBrush, area: Rect) -> Target {
        let target = Target::new();
        target.draw(|context| {
            context.push_render_options(RenderOptions {
                edge_mode: EdgeMode::Aliased,
                bitmap_interpolation_mode: BitmapInterpolationMode::None,
                ..RenderOptions::default()
            });
            context.draw_rectangle(Some(brush), None, RoundedRect::from_rect(area), &no_shadows());
            context.pop_render_options();
        });
        target
    }

    fn fill(spec: BrushSpec) -> Target {
        fill_with(&image_brush(spec), Rect::new(0.0, 0.0, 100.0, 100.0))
    }

    #[test]
    fn image_brush_stretch_fill_covers_the_area() {
        let target = fill(BrushSpec { stretch: Stretch::Fill, ..BrushSpec::default() });

        assert_eq!(RED, target.pixel(25, 5));
        assert_eq!(RED, target.pixel(25, 95));
        assert_eq!(BLUE, target.pixel(75, 5));
        assert_eq!(BLUE, target.pixel(75, 95));
    }

    #[test]
    fn image_brush_stretch_uniform_keeps_the_aspect_ratio() {
        // 20x10 scaled by 5 is 100x50, centered vertically.
        let target = fill(BrushSpec::default());

        assert_eq!(RED, target.pixel(25, 50));
        assert_eq!(BLUE, target.pixel(75, 50));
        assert_eq!(RED, target.pixel(25, 27));
        assert_eq!(RED, target.pixel(25, 72));
        assert_eq!(TRANSPARENT, target.pixel(25, 22));
        assert_eq!(TRANSPARENT, target.pixel(75, 78));
    }

    #[test]
    fn image_brush_stretch_uniform_to_fill_crops() {
        // 20x10 scaled by 10 is 200x100; the middle 100 pixels are visible.
        let target = fill(BrushSpec { stretch: Stretch::UniformToFill, ..BrushSpec::default() });

        assert_eq!(RED, target.pixel(10, 5));
        assert_eq!(RED, target.pixel(45, 95));
        assert_eq!(BLUE, target.pixel(55, 5));
        assert_eq!(BLUE, target.pixel(90, 95));
    }

    #[test]
    fn image_brush_stretch_none_follows_the_alignment() {
        let aligned = |alignment_x, alignment_y| {
            fill(BrushSpec { stretch: Stretch::None, alignment_x, alignment_y, ..BrushSpec::default() })
        };

        let top_left = aligned(AlignmentX::Left, AlignmentY::Top);
        assert_eq!(RED, top_left.pixel(5, 5));
        assert_eq!(BLUE, top_left.pixel(15, 5));
        assert_eq!(TRANSPARENT, top_left.pixel(25, 5));
        assert_eq!(TRANSPARENT, top_left.pixel(5, 15));

        let center = aligned(AlignmentX::Center, AlignmentY::Center);
        assert_eq!(RED, center.pixel(45, 50));
        assert_eq!(BLUE, center.pixel(55, 50));
        assert_eq!(TRANSPARENT, center.pixel(35, 50));
        assert_eq!(TRANSPARENT, center.pixel(50, 40));

        let bottom_right = aligned(AlignmentX::Right, AlignmentY::Bottom);
        assert_eq!(RED, bottom_right.pixel(85, 95));
        assert_eq!(BLUE, bottom_right.pixel(95, 95));
        assert_eq!(TRANSPARENT, bottom_right.pixel(75, 95));
        assert_eq!(TRANSPARENT, bottom_right.pixel(95, 85));
    }

    #[test]
    fn image_brush_tile_modes() {
        let tiled = |tile_mode| {
            fill(BrushSpec {
                stretch: Stretch::Fill,
                tile_mode,
                destination_rect: Some(RelativeRect::new(0.0, 0.0, 20.0, 10.0, RelativeUnit::Absolute)),
                ..BrushSpec::default()
            })
        };

        // Not tiled: a single tile in the destination rect.
        let none = tiled(TileMode::None);
        assert_eq!(RED, none.pixel(5, 5));
        assert_eq!(BLUE, none.pixel(15, 5));
        assert_eq!(TRANSPARENT, none.pixel(25, 5));
        assert_eq!(TRANSPARENT, none.pixel(5, 15));

        // Tile: the image repeats as it is.
        let tile = tiled(TileMode::Tile);
        assert_eq!(RED, tile.pixel(5, 5));
        assert_eq!(BLUE, tile.pixel(15, 5));
        assert_eq!(RED, tile.pixel(25, 5));
        assert_eq!(BLUE, tile.pixel(35, 5));
        assert_eq!(RED, tile.pixel(25, 15));
        assert_eq!(BLUE, tile.pixel(95, 95));

        // FlipX: every other column of tiles is mirrored.
        let flip_x = tiled(TileMode::FlipX);
        assert_eq!(RED, flip_x.pixel(5, 5));
        assert_eq!(BLUE, flip_x.pixel(15, 5));
        assert_eq!(BLUE, flip_x.pixel(25, 5));
        assert_eq!(RED, flip_x.pixel(35, 5));
        // Rows are not mirrored.
        assert_eq!(RED, flip_x.pixel(5, 15));

        // FlipY mirrors rows only, which this image does not show.
        let flip_y = tiled(TileMode::FlipY);
        assert_eq!(RED, flip_y.pixel(25, 5));
        assert_eq!(BLUE, flip_y.pixel(35, 15));

        let flip_xy = tiled(TileMode::FlipXY);
        assert_eq!(BLUE, flip_xy.pixel(25, 15));
        assert_eq!(RED, flip_xy.pixel(35, 15));
    }

    #[test]
    fn image_brush_source_rect_selects_a_part_of_the_image() {
        // The right (blue) half only.
        let target = fill(BrushSpec {
            stretch: Stretch::Fill,
            source_rect: Some(RelativeRect::new(10.0, 0.0, 10.0, 10.0, RelativeUnit::Absolute)),
            ..BrushSpec::default()
        });
        assert_eq!(BLUE, target.pixel(5, 5));
        assert_eq!(BLUE, target.pixel(95, 95));

        // The left half, relative.
        let target = fill(BrushSpec {
            stretch: Stretch::Fill,
            source_rect: Some(RelativeRect::new(0.0, 0.0, 0.5, 1.0, RelativeUnit::Relative)),
            ..BrushSpec::default()
        });
        assert_eq!(RED, target.pixel(5, 5));
        assert_eq!(RED, target.pixel(95, 95));
    }

    #[test]
    fn image_brush_destination_rect_places_the_tile() {
        // The bottom right quadrant, relative to the painted area.
        let target = fill(BrushSpec {
            stretch: Stretch::Fill,
            destination_rect: Some(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative)),
            ..BrushSpec::default()
        });

        assert_eq!(RED, target.pixel(60, 75));
        assert_eq!(BLUE, target.pixel(90, 75));
        assert_eq!(TRANSPARENT, target.pixel(25, 25));
        assert_eq!(TRANSPARENT, target.pixel(75, 25));
        assert_eq!(TRANSPARENT, target.pixel(25, 75));
    }

    #[test]
    fn image_brush_is_relative_to_the_painted_area() {
        // The same brush fills whatever it paints: here a 40x40 square away
        // from the origin.
        let target = fill_with(
            &image_brush(BrushSpec { stretch: Stretch::Fill, ..BrushSpec::default() }),
            Rect::new(50.0, 50.0, 40.0, 40.0),
        );

        assert_eq!(RED, target.pixel(55, 70));
        assert_eq!(BLUE, target.pixel(85, 70));
        assert_eq!(TRANSPARENT, target.pixel(45, 70));
    }

    #[test]
    fn image_brush_opacity_and_transform() {
        let faded = fill(BrushSpec { stretch: Stretch::Fill, opacity: 0.5, ..BrushSpec::default() });
        let (r, _, _, a) = faded.pixel(25, 50);
        assert!((a as i32 - 127).abs() <= 1 && (r as i32 - 127).abs() <= 1, "{:?}", faded.pixel(25, 50));

        let moved = fill(BrushSpec {
            stretch: Stretch::Fill,
            transform: Some(Matrix::create_translation(50.0, 0.0)),
            ..BrushSpec::default()
        });
        // The tile moved right by half the area: its red half is now on the
        // right, and nothing is left of it.
        assert_eq!(RED, moved.pixel(75, 50));
        assert_eq!(TRANSPARENT, moved.pixel(25, 50));
    }

    #[test]
    fn image_brush_without_a_source_paints_nothing() {
        let brush = ImmutableImageBrush::from_bitmap(None);
        let target = fill_with(&brush, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(TRANSPARENT, target.pixel(50, 50));
    }

    #[test]
    fn image_brush_can_stroke() {
        let brush: Rc<ImmutableImageBrush> =
            Rc::new(image_brush(BrushSpec { stretch: Stretch::Fill, ..BrushSpec::default() }));
        let pen = ImmutablePen::with_brush(Some(brush), 10.0);

        let target = Target::new();
        target.draw(|context| {
            context.push_render_options(aliased());
            context.draw_rectangle(None, Some(&pen), rect(10.0, 10.0, 80.0, 80.0), &no_shadows());
            context.pop_render_options();
        });

        // The brush fills the stroked bounds: red on the left, blue on the
        // right.
        assert_eq!(RED, target.pixel(10, 50));
        assert_eq!(RED, target.pixel(30, 10));
        assert_eq!(BLUE, target.pixel(90, 50));
        assert_eq!(BLUE, target.pixel(70, 90));
        assert_eq!(TRANSPARENT, target.pixel(50, 50));
    }

    // -----------------------------------------------------------------------
    // Scene brushes
    // -----------------------------------------------------------------------

    /// Scene brush content that draws a red and a blue 10x10 square side by
    /// side.
    struct TwoSquares {
        parameters: Rc<ImmutableSceneBrush>,
        scalable: bool,
        disposed: Rc<Cell<bool>>,
    }

    macro_rules! brush_members {
        () => {
            fn opacity(&self) -> f64 {
                self.parameters.opacity()
            }
            fn transform(&self) -> Option<Rc<dyn ITransform>> {
                None
            }
            fn transform_origin(&self) -> RelativePoint {
                RelativePoint::default()
            }
            fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
                None
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        };
    }

    impl IBrush for TwoSquares {
        brush_members!();
    }

    impl IImmutableBrush for TwoSquares {}

    impl ISceneBrushContent for TwoSquares {
        fn brush(&self) -> Rc<dyn ITileBrush> {
            self.parameters.clone()
        }
        fn rect(&self) -> Rect {
            Rect::new(0.0, 0.0, 20.0, 10.0)
        }
        fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Option<Matrix>) {
            if let Some(transform) = transform {
                context.set_transform(transform);
            }
            context.push_render_options(aliased());
            context.draw_rectangle(Some(&solid(Colors::RED)), None, rect(0.0, 0.0, 10.0, 10.0), &no_shadows());
            context.draw_rectangle(Some(&solid(Colors::BLUE)), None, rect(10.0, 0.0, 10.0, 10.0), &no_shadows());
            context.pop_render_options();
        }
        fn use_scalable_rasterization(&self) -> bool {
            self.scalable
        }
        fn dispose(&self) {
            self.disposed.set(true);
        }
    }

    struct TwoSquaresBrush {
        parameters: Rc<ImmutableSceneBrush>,
        scalable: bool,
        has_content: bool,
        disposed: Rc<Cell<bool>>,
    }

    impl IBrush for TwoSquaresBrush {
        brush_members!();

        fn as_tile_brush(&self) -> Option<&dyn ITileBrush> {
            Some(self)
        }
        fn as_scene_brush(&self) -> Option<&dyn ISceneBrush> {
            Some(self)
        }
    }

    impl ITileBrush for TwoSquaresBrush {
        fn alignment_x(&self) -> AlignmentX {
            self.parameters.alignment_x()
        }
        fn alignment_y(&self) -> AlignmentY {
            self.parameters.alignment_y()
        }
        fn destination_rect(&self) -> RelativeRect {
            self.parameters.destination_rect()
        }
        fn source_rect(&self) -> RelativeRect {
            self.parameters.source_rect()
        }
        fn stretch(&self) -> Stretch {
            self.parameters.stretch()
        }
        fn tile_mode(&self) -> TileMode {
            self.parameters.tile_mode()
        }
    }

    impl ISceneBrush for TwoSquaresBrush {
        fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
            if !self.has_content {
                return None;
            }
            Some(Rc::new(TwoSquares {
                parameters: self.parameters.clone(),
                scalable: self.scalable,
                disposed: self.disposed.clone(),
            }))
        }
    }

    fn scene_brush(spec: BrushSpec, scalable: bool, has_content: bool) -> TwoSquaresBrush {
        let parameters = ImmutableImageBrush::new(
            None,
            spec.alignment_x,
            spec.alignment_y,
            spec.destination_rect,
            spec.opacity,
            None,
            RelativePoint::default(),
            spec.source_rect,
            spec.stretch,
            spec.tile_mode,
            None,
        );
        TwoSquaresBrush {
            parameters: Rc::new(ImmutableSceneBrush::new(&parameters)),
            scalable,
            has_content,
            disposed: Rc::new(Cell::new(false)),
        }
    }

    #[test]
    fn scene_brush_content_is_rendered_through_a_surface_or_a_picture() {
        for scalable in [false, true] {
            // Stretched over the whole area.
            let brush = scene_brush(BrushSpec { stretch: Stretch::Fill, ..BrushSpec::default() }, scalable, true);
            let target = fill_with(&brush, Rect::new(0.0, 0.0, 100.0, 100.0));
            assert_eq!(RED, target.pixel(25, 10), "scalable: {scalable}");
            assert_eq!(BLUE, target.pixel(75, 90), "scalable: {scalable}");
            assert!(brush.disposed.get(), "the content is released, scalable: {scalable}");

            // Tiled with an absolute 20x10 tile.
            let brush = scene_brush(
                BrushSpec {
                    stretch: Stretch::Fill,
                    tile_mode: TileMode::Tile,
                    destination_rect: Some(RelativeRect::new(0.0, 0.0, 20.0, 10.0, RelativeUnit::Absolute)),
                    ..BrushSpec::default()
                },
                scalable,
                true,
            );
            let target = fill_with(&brush, Rect::new(0.0, 0.0, 100.0, 100.0));
            assert_eq!(RED, target.pixel(5, 5), "scalable: {scalable}");
            assert_eq!(BLUE, target.pixel(15, 5), "scalable: {scalable}");
            assert_eq!(RED, target.pixel(45, 35), "scalable: {scalable}");
            assert_eq!(BLUE, target.pixel(55, 35), "scalable: {scalable}");

            // The source rect picks the blue square.
            let brush = scene_brush(
                BrushSpec {
                    stretch: Stretch::Fill,
                    source_rect: Some(RelativeRect::new(0.5, 0.0, 0.5, 1.0, RelativeUnit::Relative)),
                    ..BrushSpec::default()
                },
                scalable,
                true,
            );
            let target = fill_with(&brush, Rect::new(0.0, 0.0, 100.0, 100.0));
            assert_eq!(BLUE, target.pixel(10, 50), "scalable: {scalable}");
            assert_eq!(BLUE, target.pixel(90, 50), "scalable: {scalable}");
        }
    }

    #[test]
    fn scene_brush_without_content_paints_nothing() {
        let brush = scene_brush(BrushSpec::default(), false, false);
        let target = fill_with(&brush, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(TRANSPARENT, target.pixel(50, 50));
    }

    // -----------------------------------------------------------------------
    // Acrylic
    // -----------------------------------------------------------------------

    struct Material {
        background_source: AcrylicBackgroundSource,
        tint_color: Color,
        material_color: Color,
    }

    impl IExperimentalAcrylicMaterial for Material {
        fn background_source(&self) -> AcrylicBackgroundSource {
            self.background_source
        }
        fn tint_color(&self) -> Color {
            self.tint_color
        }
        fn tint_opacity(&self) -> f64 {
            1.0
        }
        fn material_color(&self) -> Color {
            self.material_color
        }
        fn fallback_color(&self) -> Color {
            Colors::GRAY
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn acrylic_rectangle_is_tinted() {
        // An opaque tint hides the material colour; the noise on top is
        // barely visible.
        let target = Target::new();
        draw(&target, |context| {
            let material = Material {
                background_source: AcrylicBackgroundSource::None,
                tint_color: Colors::BLUE,
                material_color: Colors::RED,
            };
            context.draw_rectangle_with_material(&material, rect(10.0, 10.0, 80.0, 80.0));
            // Degenerate rectangles are ignored.
            context.draw_rectangle_with_material(&material, rect(0.0, 0.0, 0.0, 10.0));
        });

        let (r, g, b, a) = target.pixel(50, 50);
        assert!(a == 255 && b > 240 && r < 12 && g < 12, "tinted blue: {:?}", (r, g, b, a));
        assert_eq!(TRANSPARENT, target.pixel(5, 5));

        // The noise makes the fill slightly uneven.
        let distinct: std::collections::HashSet<_> = (20..80).map(|x| target.pixel(x, 50)).collect();
        assert!(distinct.len() > 1, "the noise texture is applied");
    }

    #[test]
    fn acrylic_translucent_tint_lets_the_material_colour_through() {
        let target = Target::new();
        draw(&target, |context| {
            let material = Material {
                background_source: AcrylicBackgroundSource::None,
                tint_color: Color::from_argb(128, 0, 0, 255),
                material_color: Colors::RED,
            };
            context.draw_rectangle_with_material(
                &material,
                RoundedRect::from_radius(Rect::new(0.0, 0.0, 100.0, 100.0), 30.0),
            );
        });

        let (r, _, b, a) = target.pixel(50, 50);
        assert!(a == 255 && (r as i32 - 127).abs() < 12 && (b as i32 - 128).abs() < 12, "{:?}", target.pixel(50, 50));
        // Rounded corners.
        assert_eq!(TRANSPARENT, target.pixel(2, 2));
    }

    #[test]
    fn acrylic_digger_replaces_what_is_underneath() {
        let material = |background_source| Material {
            background_source,
            tint_color: Color::from_argb(0, 0, 0, 0),
            material_color: Color::from_argb(0, 0, 0, 0),
        };

        // Blended normally a transparent material leaves the background.
        let blended = Target::new();
        draw(&blended, |context| {
            context.clear(Colors::RED);
            context.draw_rectangle_with_material(&material(AcrylicBackgroundSource::None), rect(0.0, 0.0, 100.0, 100.0));
        });
        let (r, _, _, a) = blended.pixel(50, 50);
        assert!(a == 255 && r > 240, "the background shows through: {:?}", blended.pixel(50, 50));

        // The digger cuts through it.
        let dug = Target::new();
        draw(&dug, |context| {
            context.clear(Colors::RED);
            context.draw_rectangle_with_material(&material(AcrylicBackgroundSource::Digger), rect(0.0, 0.0, 50.0, 100.0));
        });
        assert!(dug.pixel(25, 50).3 < 12, "the material replaced the background: {:?}", dug.pixel(25, 50));
        assert_eq!(RED, dug.pixel(75, 50));
    }
}
