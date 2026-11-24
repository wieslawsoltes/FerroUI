//! The fast paths of rounded rectangles and rounded clips draw the pixels of the general paths
//! (desktop-performance.md, the Skia drawing context's rounded rectangles).

use crate::helpers::drawing_context_helper::wrap_skia_surface;
use ferroui_base::media::{BoxShadow, BoxShadows, Brushes, Color, IBrush, IPen, Pen};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Rect, RoundedRect, Vector};
use skia_safe::{surfaces, ClipOp, RRect, Surface};
use std::rc::Rc;

fn surface() -> Surface {
    let mut surface = surfaces::raster_n32_premul((100, 100)).expect("a raster surface");
    surface.canvas().clear(skia_safe::Color::WHITE);
    surface
}

fn pixels(surface: &mut Surface) -> Vec<u8> {
    let pixmap = surface.peek_pixels().expect("the pixels of a raster surface");
    pixmap.bytes().expect("the bytes of the pixels").to_vec()
}

/// An outer shadow far outside the surface and transparent: it draws nothing here, and a rectangle with a shadow
/// takes the general path of four radii.
fn invisible_shadow() -> BoxShadows {
    BoxShadows::new(BoxShadow {
        offset_x: 1000.0,
        offset_y: 1000.0,
        color: Color::from_argb(0, 0, 0, 0),
        ..BoxShadow::default()
    })
}

fn draw(surface: &Surface, rect: RoundedRect, shadows: &BoxShadows) {
    let mut context = wrap_skia_surface(surface, Vector::new(96.0, 96.0));
    let brush: Rc<dyn IBrush> = Brushes::red();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::blue()), 3.0).into();
    context.draw_rectangle(Some(&*brush), Some(&*pen), rect, shadows);
    context.dispose();
}

#[test]
fn a_rounded_rectangle_with_four_equal_corners_draws_the_pixels_of_four_radii() {
    let bounds = Rect::new(10.5, 10.25, 60.3, 40.7);
    for (rx, ry) in [(6.0, 6.0), (8.5, 4.25), (30.15, 20.35)] {
        let rect = RoundedRect::from_radius_xy(bounds, rx, ry);
        let mut fast = surface();
        draw(&fast, rect, &BoxShadows::default());
        let mut general = surface();
        draw(&general, rect, &invisible_shadow());
        assert!(pixels(&fast) == pixels(&general), "radii {rx}, {ry}");
    }
}

#[test]
fn radii_that_do_not_fit_take_the_general_path() {
    let bounds = Rect::new(10.0, 10.0, 40.0, 20.0);
    let rect = RoundedRect::from_radius_xy(bounds, 25.0, 15.0);
    let rc = crate::skia_sharp_extensions::to_sk_rect(bounds);
    assert_eq!(crate::drawing_context_impl::uniform_radii(&rc, &rect), None);
    let mut fast = surface();
    draw(&fast, rect, &BoxShadows::default());
    let mut general = surface();
    draw(&general, rect, &invisible_shadow());
    assert!(pixels(&fast) == pixels(&general));
}

/// Fills the surface red through a drawing context under the clip of the canvas.
fn fill_under_clip(surface: &Surface, push: impl FnOnce(&mut dyn IDrawingContextImpl)) {
    let mut context = wrap_skia_surface(surface, Vector::new(96.0, 96.0));
    push(&mut context);
    let brush: Rc<dyn IBrush> = Brushes::red();
    let whole = RoundedRect::from(Rect::new(0.0, 0.0, 100.0, 100.0));
    context.draw_rectangle(Some(&*brush), None, whole, &BoxShadows::default());
    context.dispose();
}

#[test]
fn a_rounded_clip_without_rounding_clips_as_the_rounded_clip_of_a_rectangle() {
    let bounds = Rect::new(10.5, 10.25, 60.3, 40.7);
    let rc = crate::skia_sharp_extensions::to_sk_rect(bounds);
    let mut fast = surface();
    fill_under_clip(&fast, |context| context.push_clip_rounded(RoundedRect::from(bounds)));
    let mut general = surface();
    general.canvas().save();
    general.canvas().clip_rrect(RRect::new_rect(rc), ClipOp::Intersect, true);
    fill_under_clip(&general, |_| {});
    general.canvas().restore();
    assert!(pixels(&fast) == pixels(&general));
}

#[test]
fn a_rounded_clip_with_four_equal_corners_clips_as_four_radii() {
    let bounds = Rect::new(10.5, 10.25, 60.3, 40.7);
    let rc = crate::skia_sharp_extensions::to_sk_rect(bounds);
    let mut fast = surface();
    fill_under_clip(&fast, |context| context.push_clip_rounded(RoundedRect::from_radius_xy(bounds, 9.0, 5.5)));
    let mut general = surface();
    general.canvas().save();
    let radii = [skia_safe::Vector::new(9.0, 5.5); 4];
    general.canvas().clip_rrect(RRect::new_rect_radii(rc, &radii), ClipOp::Intersect, true);
    fill_under_clip(&general, |_| {});
    general.canvas().restore();
    assert!(pixels(&fast) == pixels(&general));
}
