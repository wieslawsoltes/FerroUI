//! Port of upstream's `DrawingContextImplTests.cs` of the Skia unit tests.
//!
//! Upstream wraps the canvas of a 100x100 `SKBitmap` with
//! `DrawingContextHelper.WrapSkiaCanvas`; the port's helper wraps the canvas
//! of a raster surface of the same size.

use crate::helpers::drawing_context_helper::wrap_skia_surface;
use crate::DrawingContextImpl;
use ferroui_base::media::{BoxShadows, Brushes, IBrush, IPen, Pen};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{CornerRadius, Point, Rect, RoundedRect, Vector};
use std::rc::Rc;

#[test]
fn draw_line_with_zero_thickness_pen_does_not_throw() {
    let (mut target, _surface) = create_target();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 0.0).into();
    target.draw_line(Some(&*pen), Point::new(0.0, 0.0), Point::new(10.0, 10.0));
}

#[test]
fn draw_rectangle_with_zero_thickness_pen_does_not_throw() {
    let (mut target, _surface) = create_target();
    let brush: Rc<dyn IBrush> = Brushes::black();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 0.0).into();
    target.draw_rectangle(
        Some(&*brush),
        Some(&*pen),
        RoundedRect::from_corner_radius(Rect::new(0.0, 0.0, 100.0, 100.0), CornerRadius::uniform(4.0)),
        &BoxShadows::default(),
    );
}

fn create_target() -> (DrawingContextImpl, skia_safe::Surface) {
    let surface = skia_safe::surfaces::raster_n32_premul((100, 100)).unwrap();
    let target = wrap_skia_surface(&surface, Vector::new(96.0, 96.0));
    (target, surface)
}
