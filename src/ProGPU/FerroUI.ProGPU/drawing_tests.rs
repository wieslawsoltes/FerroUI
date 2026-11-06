//! The drawing context on the device: what is drawn is checked in the read-back pixels.

use crate::geometry::{PathData, ProGpuGeometry};
use crate::{ProGpuDevice, ProGpuDrawingContext, SceneBuilder};
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadow, BoxShadows, Color, IBrush, PenLineCap, PenLineJoin};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Matrix, Point, Rect, RoundedRect, Vector};
use std::rc::Rc;

const SIZE: u32 = 96;

fn device() -> Option<ProGpuDevice> {
    match ProGpuDevice::new() {
        Ok(device) => Some(device),
        Err(error) => {
            eprintln!("skipped: {error}");
            None
        }
    }
}

/// Draws with `draw` on a white target and returns the read-back RGBA8 pixels.
fn render(device: &ProGpuDevice, draw: impl FnOnce(&mut ProGpuDrawingContext)) -> Vec<u8> {
    let mut context = ProGpuDrawingContext::new(SceneBuilder::new(), None);
    context.clear(Color::from_rgb(255, 255, 255));
    draw(&mut context);
    let (mut scene, error) = context.finish();
    assert!(error.is_none(), "recording: {error:?}");
    let stream = scene.build().expect("the stream of the scene").to_vec();
    device.update_scene(&stream).expect("the scene");
    let (texture, view) = device.create_target(SIZE, SIZE);
    device.render_scene(scene.scene_id(), scene.generation(), view, SIZE, SIZE, 1.0).expect("the frame");
    device.read_rgba8(texture, SIZE, SIZE)
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
