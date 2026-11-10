//! Tests of when a transform of the drawing context reaches the canvas
//! (desktop-performance.md, 9.28). Not from upstream, which sets every
//! transform on the canvas at once.

use crate::drawing_context_impl::{CanvasSource, CreateInfo};
use crate::helpers::drawing_context_helper::wrap_skia_surface;
use crate::skia_sharp_extensions::to_sk_matrix44;
use crate::{DrawingContextImpl, ISkiaApiLeaseFeature};
use ferroui_base::media::{BoxShadows, Brushes, IBrush};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Matrix, Rect, RoundedRect, Vector};
use skia_safe::{surfaces, Color, Picture, PictureRecorder, Surface};
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

fn white_surface() -> Surface {
    let mut surface = surfaces::raster_n32_premul((100, 100)).expect("a raster surface");
    surface.canvas().clear(Color::WHITE);
    surface
}

fn color(surface: &mut Surface, x: i32, y: i32) -> Color {
    surface.peek_pixels().expect("the pixels of a raster surface").get_color((x, y))
}

fn fill(context: &mut DrawingContextImpl, brush: &Rc<dyn IBrush>, rect: Rect) {
    context.draw_rectangle(Some(&**brush), None, RoundedRect::from(rect), &BoxShadows::default());
}

/// The picture of what `draw` draws through a drawing context.
fn record(draw: impl FnOnce(&mut DrawingContextImpl)) -> Picture {
    let mut recorder = PictureRecorder::new();
    recorder.begin_recording(skia_safe::Rect::from_wh(100.0, 100.0), false);
    let picture = Rc::new(RefCell::new(None));
    let recorded = picture.clone();
    let mut context = DrawingContextImpl::new(
        CreateInfo {
            canvas: Some(CanvasSource::Recorder {
                recorder,
                on_finished: Box::new(move |picture| *recorded.borrow_mut() = picture),
            }),
            dpi: Vector::new(96.0, 96.0),
            disable_subpixel_text_rendering: true,
            ..CreateInfo::default()
        },
        Vec::new(),
    );
    draw(&mut context);
    context.dispose();
    let picture = picture.borrow_mut().take();
    picture.expect("a picture")
}

#[test]
fn drawing_uses_the_last_transform_set() {
    let mut surface = white_surface();
    let mut context = wrap_skia_surface(&surface, Vector::new(96.0, 96.0));
    let red: Rc<dyn IBrush> = Brushes::red();
    let green: Rc<dyn IBrush> = Brushes::green();
    let blue: Rc<dyn IBrush> = Brushes::blue();
    let square = Rect::new(0.0, 0.0, 10.0, 10.0);

    // Replaced before anything is drawn with it.
    context.set_transform(Matrix::create_translation(50.0, 0.0));
    context.set_transform(Matrix::create_translation(10.0, 10.0));
    fill(&mut context, &red, square);

    // The clip is in the coordinates of the transform before it; the
    // transform set inside it ends with it.
    context.push_clip(Rect::new(0.0, 0.0, 5.0, 5.0));
    context.set_transform(Matrix::create_translation(60.0, 60.0));
    fill(&mut context, &blue, square);
    context.set_transform(Matrix::create_translation(80.0, 80.0));
    context.pop_clip();
    assert_eq!(Matrix::create_translation(10.0, 10.0), context.transform());
    fill(&mut context, &green, Rect::new(20.0, 0.0, 10.0, 10.0));

    // Set and never drawn with: the surface is left with it.
    context.set_transform(Matrix::create_translation(70.0, 70.0));
    assert_eq!(Matrix::create_translation(70.0, 70.0), context.transform());
    context.dispose();

    assert_eq!(Color::RED, color(&mut surface, 15, 15));
    assert_eq!(Color::WHITE, color(&mut surface, 55, 5));
    assert_eq!(Color::WHITE, color(&mut surface, 65, 65));
    assert_eq!(Color::WHITE, color(&mut surface, 85, 85));
    assert_eq!(Color::from_rgb(0, 128, 0), color(&mut surface, 35, 15));
    assert_eq!(to_sk_matrix44(Matrix::create_translation(70.0, 70.0)), surface.canvas().local_to_device());
}

#[test]
fn a_transform_that_nothing_is_drawn_with_is_not_recorded() {
    let red: Rc<dyn IBrush> = Brushes::red();
    let square = Rect::new(0.0, 0.0, 10.0, 10.0);
    let first = Matrix::create_translation(10.0, 10.0);
    let second = Matrix::create_translation(30.0, 30.0);

    // What the walk over two sibling visuals sets: the transform of the
    // first, the one of the parent after it, the one of the second, the
    // one of the parent again.
    let walked = record(|context| {
        context.set_transform(first);
        fill(context, &red, square);
        context.set_transform(Matrix::IDENTITY);
        context.set_transform(second);
        fill(context, &red, square);
        context.set_transform(Matrix::IDENTITY);
    });
    // Only the transforms something is drawn with.
    let needed = record(|context| {
        context.set_transform(first);
        fill(context, &red, square);
        context.set_transform(second);
        fill(context, &red, square);
    });
    assert_eq!(needed.approximate_op_count(), walked.approximate_op_count());

    // And the picture draws what the walk asked for.
    let mut surface = white_surface();
    surface.canvas().draw_picture(&walked, None, None);
    assert_eq!(Color::RED, color(&mut surface, 15, 15));
    assert_eq!(Color::RED, color(&mut surface, 35, 35));
    assert_eq!(Color::WHITE, color(&mut surface, 5, 5));
    assert_eq!(Color::WHITE, color(&mut surface, 25, 25));
}

#[test]
fn the_canvas_follows_the_transform_at_once_when_the_api_can_be_leased() {
    let mut surface = white_surface();
    let mut context = DrawingContextImpl::new(
        CreateInfo {
            surface: Some(surface.clone()),
            dpi: Vector::new(96.0, 96.0),
            disable_subpixel_text_rendering: true,
            ..CreateInfo::default()
        },
        Vec::new(),
    );
    let earlier = Matrix::create_translation(5.0, 5.0);
    context.set_transform(earlier);
    assert!(context.get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>()).is_some());
    // The transform set before the feature was asked for.
    assert_eq!(to_sk_matrix44(earlier), surface.canvas().local_to_device());

    let later = Matrix::create_translation(20.0, 30.0);
    context.set_transform(later);
    assert_eq!(to_sk_matrix44(later), surface.canvas().local_to_device());
    context.dispose();
}
