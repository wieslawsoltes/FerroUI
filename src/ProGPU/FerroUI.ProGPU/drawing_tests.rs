//! The drawing context on the device: what is drawn is checked in the read-back pixels.

use crate::geometry::{PathData, ProGpuGeometry};
use crate::{ProGpuDevice, ProGpuDrawingContext, ProGpuEngine, SceneBuilder, TargetFormat};
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadow, BoxShadows, Color, GradientSpreadMethod, IBrush, PenLineCap, PenLineJoin};
<<<<<<< HEAD
use ferroui_base::media::effects::{ImmutableBlurEffect, ImmutableDropShadowEffect};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextImplWithEffects};
=======
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextImplWithFragments, IDrawingFragment};
>>>>>>> progpu/p5-image-formats
use ferroui_base::{Matrix, Point, Rect, RelativePoint, RelativeUnit, RoundedRect, Vector};
use std::rc::Rc;

const SIZE: u32 = 96;

fn device() -> Option<ProGpuEngine> {
    match ProGpuDevice::new().and_then(|device| ProGpuEngine::new(&Rc::new(device), TargetFormat::Rgba8)) {
        Ok(engine) => Some(engine),
        Err(error) => {
            eprintln!("skipped: {error}");
            None
        }
    }
}

/// Draws with `draw` on a white target and returns the read-back RGBA8 pixels.
fn render(engine: &ProGpuEngine, draw: impl FnOnce(&mut ProGpuDrawingContext)) -> Vec<u8> {
    let mut context = ProGpuDrawingContext::new(SceneBuilder::new(), None);
    context.clear(Color::from_rgb(255, 255, 255));
    draw(&mut context);
    let (mut scene, error) = context.finish();
    assert!(error.is_none(), "recording: {error:?}");
    let stream = scene.build().expect("the stream of the scene").to_vec();
    engine.update_scene(&stream).expect("the scene");
    let device = engine.device();
    let (texture, view) = device.create_target(SIZE, SIZE);
    engine.render_scene(scene.scene_id(), scene.generation(), view, SIZE, SIZE, false).expect("the frame");
    let pixels = device.read_rgba8(texture, SIZE, SIZE);
    device.release_texture(texture, view);
    pixels
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let index = ((y * SIZE + x) * 4) as usize;
    [pixels[index], pixels[index + 1], pixels[index + 2], pixels[index + 3]]
}

fn solid(r: u8, g: u8, b: u8) -> Rc<ImmutableSolidColorBrush> {
    Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(r, g, b)))
}

#[test]
fn rectangles_ellipses_and_transforms() {
    let Some(device) = device() else { return };
    let red = solid(255, 0, 0);
    let blue = solid(0, 0, 255);
    let pixels = render(&device, |context| {
        context.draw_rectangle(Some(red.as_ref() as &dyn IBrush), None, RoundedRect::from(Rect::new(8.0, 8.0, 32.0, 16.0)), &BoxShadows::default());
        context.set_transform(Matrix::create_translation(48.0, 48.0));
        context.draw_ellipse(Some(blue.as_ref() as &dyn IBrush), None, Rect::new(0.0, 0.0, 40.0, 40.0));
    });
    assert_eq!([255, 0, 0, 255], pixel(&pixels, 20, 16), "the rectangle");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 4, 4), "the cleared target");
    assert_eq!([0, 0, 255, 255], pixel(&pixels, 68, 68), "the translated ellipse");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 49, 49), "outside the ellipse, in its box");
}

#[test]
fn clips_restrict_drawing() {
    let Some(device) = device() else { return };
    let green = solid(0, 160, 0);
    let pixels = render(&device, |context| {
        context.push_clip(Rect::new(0.0, 0.0, 48.0, 96.0));
        context.push_clip_rounded(RoundedRect::from_uniform_radii(Rect::new(8.0, 8.0, 80.0, 80.0), Vector::new(20.0, 20.0)));
        context.draw_rectangle(Some(green.as_ref() as &dyn IBrush), None, RoundedRect::from(Rect::new(0.0, 0.0, 96.0, 96.0)), &BoxShadows::default());
        context.pop_clip();
        context.pop_clip();
    });
    assert_eq!([0, 160, 0, 255], pixel(&pixels, 30, 48), "inside both clips");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 60, 48), "outside the rectangle clip");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 10, 10), "outside the rounded corner");
}

#[test]
fn box_shadows_stay_outside_a_translucent_element() {
    let Some(device) = device() else { return };
    let translucent = Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(0, 255, 255, 255)));
    let shadows = BoxShadows::new(BoxShadow {
        offset_x: 0.0,
        offset_y: 0.0,
        blur: 16.0,
        spread: 0.0,
        color: Color::from_rgb(0, 0, 0),
        is_inset: false,
    });
    let pixels = render(&device, |context| {
        context.draw_rectangle(Some(translucent.as_ref() as &dyn IBrush), None, RoundedRect::from(Rect::new(24.0, 24.0, 48.0, 48.0)), &shadows);
    });
    let outside = pixel(&pixels, 20, 48);
    assert!(outside[0] < 200, "the shadow outside the element: {outside:?}");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 48, 48), "no shadow under the element");
}

#[test]
fn geometries_fill_and_stroke() {
    let Some(device) = device() else { return };
    let black = solid(0, 0, 0);
    let pen = ImmutablePen::new(Some(solid(0, 0, 255)), 6.0, None, PenLineCap::Flat, PenLineJoin::Miter, 10.0);
    let triangle = {
        use crate::geometry::PathBuilder;
        use ferroui_base::platform::IGeometryContext;
        let mut builder = PathBuilder::default();
        builder.begin_figure(Point::new(10.0, 80.0), true);
        builder.line_to(Point::new(50.0, 10.0), true);
        builder.line_to(Point::new(90.0, 80.0), true);
        builder.end_figure(true);
        ProGpuGeometry::new(builder.finish())
    };
    let pixels = render(&device, |context| {
        context.draw_geometry(Some(black.as_ref() as &dyn IBrush), Some(&pen), triangle.as_ref());
    });
    assert_eq!([0, 0, 0, 255], pixel(&pixels, 50, 60), "the fill");
    assert_eq!([0, 0, 255, 255], pixel(&pixels, 50, 80), "the stroke on the base");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 5, 5), "outside");
    let _ = Vector::new(0.0, 0.0);
    let _ = PathData::default();
}

/// A gradient from red on the left of the box it fills to blue on its right.
fn left_to_right() -> Rc<ImmutableLinearGradientBrush> {
    Rc::new(ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0.0, Color::from_rgb(255, 0, 0)),
            ImmutableGradientStop::new(1.0, Color::from_rgb(0, 0, 255)),
        ],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.5, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.5, RelativeUnit::Relative)),
        None,
    ))
}

fn is_reddish(pixel: [u8; 4]) -> bool {
    pixel[0] > 190 && pixel[2] < 65 && pixel[1] < 10
}

fn is_bluish(pixel: [u8; 4]) -> bool {
    pixel[2] > 190 && pixel[0] < 65 && pixel[1] < 10
}

fn is_purple(pixel: [u8; 4]) -> bool {
    (100..156).contains(&pixel[0]) && (100..156).contains(&pixel[2]) && pixel[1] < 10
}

/// A gradient is laid out on the box of what it fills, wherever the transform puts that box:
/// under a translation and a scale, for the analytic shapes and for a path.
#[test]
fn gradients_follow_the_box_they_fill() {
    let Some(device) = device() else { return };
    let brush = left_to_right();
    let brush: &dyn IBrush = brush.as_ref();
    let transform = Matrix::create_scale(2.0, 2.0) * Matrix::create_translation(8.0, 8.0);

    // Local box (4, 4) to (36, 20): device (16, 16) to (80, 48).
    let shapes: [(&str, Box<dyn Fn(&mut ProGpuDrawingContext)>); 4] = [
        ("rectangle", Box::new(|context| {
            context.draw_rectangle(Some(brush), None, RoundedRect::from(Rect::new(4.0, 4.0, 32.0, 16.0)), &BoxShadows::default());
        })),
        ("rounded rectangle", Box::new(|context| {
            let rect = RoundedRect::from_uniform_radii(Rect::new(4.0, 4.0, 32.0, 16.0), Vector::new(2.0, 2.0));
            context.draw_rectangle(Some(brush), None, rect, &BoxShadows::default());
        })),
        ("ellipse", Box::new(|context| {
            context.draw_ellipse(Some(brush), None, Rect::new(4.0, 4.0, 32.0, 16.0));
        })),
        ("path", Box::new(|context| {
            let geometry = ProGpuGeometry::new(PathData::ellipse(Rect::new(4.0, 4.0, 32.0, 16.0)));
            context.draw_geometry(Some(brush), None, geometry.as_ref());
        })),
    ];
    for (name, draw) in &shapes {
        let pixels = render(&device, |context| {
            context.set_transform(transform);
            draw(context);
        });
        let (left, middle, right) = (pixel(&pixels, 20, 32), pixel(&pixels, 48, 32), pixel(&pixels, 76, 32));
        assert!(is_reddish(left), "{name}: the left of the gradient: {left:?}");
        assert!(is_purple(middle), "{name}: the middle of the gradient: {middle:?}");
        assert!(is_bluish(right), "{name}: the right of the gradient: {right:?}");
    }

    // Turned a quarter: the box is device (48, 16) to (80, 80) and the gradient runs down.
    let turned = Matrix::create_scale(2.0, 2.0) * Matrix::create_rotation(std::f64::consts::FRAC_PI_2) * Matrix::create_translation(88.0, 8.0);
    for (name, draw) in &shapes {
        let pixels = render(&device, |context| {
            context.set_transform(turned);
            draw(context);
        });
        let (top, middle, bottom) = (pixel(&pixels, 64, 20), pixel(&pixels, 64, 48), pixel(&pixels, 64, 76));
        assert!(is_reddish(top), "{name}, turned: the start of the gradient: {top:?}");
        assert!(is_purple(middle), "{name}, turned: the middle of the gradient: {middle:?}");
        assert!(is_bluish(bottom), "{name}, turned: the end of the gradient: {bottom:?}");
    }
}

/// The relative transform of a brush acts on the box as a unit square: mirrored there, the
/// gradient runs the other way.
#[test]
fn the_relative_transform_of_a_brush_acts_on_the_unit_square() {
    use ferroui_base::media::immutable::ImmutableTransform;
    let Some(device) = device() else { return };
    let mirrored = Matrix::create_scale(-1.0, 1.0) * Matrix::create_translation(1.0, 0.0);
    let brush = ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0.0, Color::from_rgb(255, 0, 0)),
            ImmutableGradientStop::new(1.0, Color::from_rgb(0, 0, 255)),
        ],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.5, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.5, RelativeUnit::Relative)),
        Some(Rc::new(ImmutableTransform::new(mirrored))),
    );
    let pixels = render(&device, |context| {
        context.draw_rectangle(Some(&brush as &dyn IBrush), None, RoundedRect::from(Rect::new(16.0, 16.0, 64.0, 32.0)), &BoxShadows::default());
    });
    let (left, right) = (pixel(&pixels, 20, 32), pixel(&pixels, 76, 32));
    assert!(is_bluish(left), "the left of the mirrored gradient: {left:?}");
    assert!(is_reddish(right), "the right of the mirrored gradient: {right:?}");
}

<<<<<<< HEAD
fn fill_rect(context: &mut ProGpuDrawingContext, brush: &dyn IBrush, rect: Rect) {
    context.draw_rectangle(Some(brush), None, RoundedRect::from_rect(rect), &BoxShadows::default());
}

#[test]
fn a_blur_spreads_its_content_over_the_edge() {
    let Some(engine) = device() else { return };
    let black = solid(0, 0, 0);
    let pixels = render(&engine, |context| {
        context.push_effect(None, &ImmutableBlurEffect::new(12.0));
        fill_rect(context, black.as_ref(), Rect::new(32.0, 32.0, 32.0, 32.0));
        context.pop_effect();
    });
    let center = pixel(&pixels, 48, 48)[0];
    let edge = pixel(&pixels, 32, 48)[0];
    let outside = pixel(&pixels, 28, 48)[0];
    let far = pixel(&pixels, 4, 48)[0];
    assert!(center < 40, "the middle stays dark: {center}");
    assert!(edge > 90 && edge < 170, "the edge is about half: {edge}");
    assert!(outside > edge && outside < 250, "the blur reaches outside: {outside}");
    assert_eq!(255, far, "nothing far away");
}

#[test]
fn a_drop_shadow_falls_at_its_offset_under_its_content() {
    let Some(engine) = device() else { return };
    let red = solid(255, 0, 0);
    let effect = ImmutableDropShadowEffect::new(12.0, 10.0, 0.0, Color::from_rgb(0, 0, 255), 1.0);
    let pixels = render(&engine, |context| {
        context.push_effect(None, &effect);
        fill_rect(context, red.as_ref(), Rect::new(20.0, 20.0, 30.0, 30.0));
        context.pop_effect();
    });
    assert_eq!([255, 0, 0, 255], pixel(&pixels, 30, 30), "the content over its shadow");
    assert_eq!([0, 0, 255, 255], pixel(&pixels, 56, 56), "the shadow at its offset");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 10, 10), "nothing elsewhere");
}

#[test]
fn a_compositing_layer_blends_its_content() {
    let Some(engine) = device() else { return };
    let (red, gray) = (solid(255, 0, 0), solid(128, 128, 128));
    let pixels = render(&engine, |context| {
        fill_rect(context, red.as_ref(), Rect::new(0.0, 0.0, 96.0, 96.0));
        assert!(context.try_push_compositing_layer(BitmapBlendingMode::Multiply, 1.0, Rect::new(0.0, 0.0, 48.0, 96.0)));
        fill_rect(context, gray.as_ref(), Rect::new(0.0, 0.0, 48.0, 96.0));
        context.pop_compositing_layer();
        assert!(!context.try_push_compositing_layer(BitmapBlendingMode::Multiply, 2.0, Rect::new(0.0, 0.0, 1.0, 1.0)));
    });
    let blended = pixel(&pixels, 24, 48);
    assert!(blended[0].abs_diff(128) <= 1 && blended[1] == 0 && blended[2] == 0, "multiplied: {blended:?}");
    assert_eq!([255, 0, 0, 255], pixel(&pixels, 72, 48), "outside the content");
}

#[test]
fn an_opacity_mask_of_a_gradient_fades_its_content() {
    let Some(engine) = device() else { return };
    let black = solid(0, 0, 0);
    let mask = ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0.0, Color::from_argb(255, 0, 0, 0)),
            ImmutableGradientStop::new(1.0, Color::from_argb(0, 0, 0, 0)),
        ],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative)),
        None,
    );
    let pixels = render(&engine, |context| {
        context.push_opacity_mask(&mask, Rect::new(0.0, 0.0, 96.0, 96.0));
        fill_rect(context, black.as_ref(), Rect::new(0.0, 0.0, 96.0, 96.0));
        context.pop_opacity_mask();
    });
    let (left, middle, right) = (pixel(&pixels, 4, 48)[0], pixel(&pixels, 48, 48)[0], pixel(&pixels, 91, 48)[0]);
    assert!(left < 30, "opaque at the start: {left}");
    assert!(middle > 100 && middle < 155, "half way: {middle}");
    assert!(right > 225, "transparent at the end: {right}");
}

#[test]
fn an_inset_shadow_darkens_the_inner_edge() {
    let Some(engine) = device() else { return };
    let white = solid(255, 255, 255);
    let shadows = BoxShadows::new(BoxShadow { offset_x: 0.0, offset_y: 0.0, blur: 8.0, spread: 0.0, color: Color::from_rgb(0, 0, 0), is_inset: true });
    let pixels = render(&engine, |context| {
        context.draw_rectangle(Some(white.as_ref() as &dyn IBrush), None, RoundedRect::from_rect(Rect::new(16.0, 16.0, 64.0, 64.0)), &shadows);
    });
    let (edge, inner, middle, outside) = (pixel(&pixels, 17, 48)[0], pixel(&pixels, 26, 48)[0], pixel(&pixels, 48, 48)[0], pixel(&pixels, 10, 48)[0]);
    assert!(edge < inner, "darker at the edge: {edge} {inner}");
    assert!(middle > 240, "the middle is untouched: {middle}");
    assert_eq!(255, outside, "nothing outside the shape");
}

#[test]
fn the_shadow_of_uneven_corners_is_drawn() {
    let Some(engine) = device() else { return };
    let white = solid(255, 255, 255);
    let rect = RoundedRect::new(
        Rect::new(24.0, 24.0, 48.0, 48.0),
        Vector::new(12.0, 12.0),
        Vector::new(0.0, 0.0),
        Vector::new(6.0, 6.0),
        Vector::new(0.0, 0.0),
    );
    let shadows = BoxShadows::new(BoxShadow { offset_x: 6.0, offset_y: 6.0, blur: 0.0, spread: 0.0, color: Color::from_rgb(0, 0, 0), is_inset: false });
    let pixels = render(&engine, |context| {
        context.draw_rectangle(Some(white.as_ref() as &dyn IBrush), None, rect, &shadows);
    });
    assert_eq!([0, 0, 0, 255], pixel(&pixels, 75, 50), "the shadow right of the shape");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 48, 48), "the shape over it");
    assert_eq!([255, 255, 255, 255], pixel(&pixels, 12, 12), "nothing else");
=======
/// The content of the fragment tests: shapes, a gradient path, a clip and a group, under the
/// transform in force.
fn fragment_content(context: &mut ProGpuDrawingContext) {
    let red = solid(210, 40, 30);
    let blue = solid(30, 60, 200);
    let red: &dyn IBrush = &*red;
    let blue: &dyn IBrush = &*blue;
    context.draw_rectangle(Some(red), None, RoundedRect::from_rect(Rect::new(4.0, 4.0, 30.0, 20.0)), &BoxShadows::default());
    context.draw_ellipse(Some(blue), None, Rect::new(10.0, 12.0, 24.0, 18.0));
    context.push_clip(Rect::new(6.0, 26.0, 16.0, 10.0));
    context.draw_rectangle(Some(blue), None, RoundedRect::from_rect(Rect::new(0.0, 20.0, 40.0, 30.0)), &BoxShadows::default());
    context.pop_clip();
    context.push_opacity(0.5, Some(Rect::new(20.0, 24.0, 20.0, 16.0)));
    context.draw_rectangle(Some(red), None, RoundedRect::from_rect(Rect::new(22.0, 26.0, 14.0, 10.0)), &BoxShadows::default());
    context.pop_opacity();
    let outline = ProGpuGeometry::new(PathData::ellipse(Rect::new(2.0, 34.0, 18.0, 10.0)));
    context.draw_geometry(Some(blue), None, outline.as_ref());
}

fn kept_fragment(engine: &ProGpuEngine, transform: Matrix) -> Rc<dyn IDrawingFragment> {
    let mut kept = None;
    render(engine, |context| {
        context.set_transform(transform);
        context.begin_fragment();
        fragment_content(context);
        kept = context.end_fragment();
    });
    kept.expect("the content can be kept")
}

#[test]
fn a_fragment_draws_what_its_commands_draw_in_place_and_moved_by_whole_pixels() {
    let Some(engine) = device() else { return };
    let transform = Matrix::create_scale(1.5, 1.5) * Matrix::create_translation(8.0, 6.0);
    let fragment = kept_fragment(&engine, transform);
    for (x, y) in [(0.0, 0.0), (7.0, 5.0), (-3.0, 12.0)] {
        let at = transform * Matrix::create_translation(x, y);
        let expected = render(&engine, |context| {
            context.set_transform(at);
            fragment_content(context);
        });
        let actual = render(&engine, |context| {
            context.set_transform(at);
            assert!(context.draw_fragment(&fragment), "moved by ({x}, {y})");
        });
        assert!(expected == actual, "moved by ({x}, {y}): the pixels differ");
    }
}

#[test]
fn a_fragment_takes_the_clip_in_force_where_it_is_drawn() {
    let Some(engine) = device() else { return };
    let transform = Matrix::create_translation(10.0, 10.0);
    let fragment = kept_fragment(&engine, transform);
    let clip = Rect::new(16.0, 12.0, 30.0, 22.0);
    let expected = render(&engine, |context| {
        context.push_clip(clip);
        context.set_transform(transform);
        fragment_content(context);
        context.set_transform(Matrix::IDENTITY);
        context.pop_clip();
    });
    let actual = render(&engine, |context| {
        context.push_clip(clip);
        context.set_transform(transform);
        assert!(context.draw_fragment(&fragment));
        context.set_transform(Matrix::IDENTITY);
        context.pop_clip();
    });
    assert!(expected == actual, "the pixels differ under the clip");
}

#[test]
fn a_fragment_is_refused_where_its_pixels_would_differ() {
    let Some(engine) = device() else { return };
    let transform = Matrix::create_translation(10.0, 10.0);
    let fragment = kept_fragment(&engine, transform);
    render(&engine, |context| {
        context.set_transform(transform * Matrix::create_translation(0.5, 0.0));
        assert!(!context.draw_fragment(&fragment), "a move by a fraction of a pixel");
        context.set_transform(Matrix::create_scale(2.0, 2.0) * transform);
        assert!(!context.draw_fragment(&fragment), "another scale");
        context.set_transform(Matrix::create_rotation(0.2) * transform);
        assert!(!context.draw_fragment(&fragment), "a rotation");
        context.set_transform(transform);
        context.begin_fragment();
        assert!(!context.draw_fragment(&fragment), "inside a fragment being recorded");
        context.end_fragment();
    });
}

#[test]
fn what_cannot_be_drawn_again_makes_no_fragment() {
    let Some(engine) = device() else { return };
    let blue = solid(30, 60, 200);
    let blue: &dyn IBrush = &*blue;
    render(&engine, |context| {
        context.begin_fragment();
        context.clear(Color::from_rgb(0, 0, 0));
        assert!(context.end_fragment().is_none(), "a clear");

        context.begin_fragment();
        context.push_clip_rounded(RoundedRect::from_radius(Rect::new(0.0, 0.0, 20.0, 20.0), 4.0));
        context.draw_rectangle(Some(blue), None, RoundedRect::from_rect(Rect::new(0.0, 0.0, 30.0, 30.0)), &BoxShadows::default());
        context.pop_clip();
        assert!(context.end_fragment().is_none(), "a rounded clip");

        context.push_clip(Rect::new(0.0, 0.0, 40.0, 40.0));
        context.begin_fragment();
        context.pop_clip();
        assert!(context.end_fragment().is_none(), "the pop of a clip opened before the fragment");
    });
>>>>>>> progpu/p5-image-formats
}
