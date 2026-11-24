//! The fill and stroke paints the drawing context keeps from draw to draw (their known values are set only
//! when they change) draw what a context with fresh paints draws, whatever brush, pen or option came before.

use crate::helpers::drawing_context_helper::wrap_skia_surface;
use crate::DrawingContextImpl;
use ferroui_base::media::immutable::{
    ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::{
    BoxShadows, Color, Colors, EdgeMode, GradientSpreadMethod, IBrush, IPen, PenLineCap, PenLineJoin, RenderOptions,
};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Point, Rect, RelativePoint, RelativeUnit, RoundedRect, Vector};
use skia_safe::{surfaces, Surface};
use std::rc::Rc;

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn gradient() -> Rc<dyn IBrush> {
    Rc::new(ImmutableLinearGradientBrush::new(
        &[ImmutableGradientStop::new(0.0, Colors::RED), ImmutableGradientStop::new(1.0, Colors::BLUE)],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)),
        None,
    ))
}

fn pen(color: Color, thickness: f64, dashed: bool, cap: PenLineCap, join: PenLineJoin) -> Rc<dyn IPen> {
    let dash = dashed.then(|| Rc::new(ImmutableDashStyle::new(Some(&[2.0, 1.0]), 0.0)));
    Rc::new(ImmutablePen::new(
        Some(Rc::new(ImmutableSolidColorBrush::new(color))),
        thickness,
        dash,
        cap,
        join,
        10.0,
    ))
}

/// One draw of the sequence.
type Draw = Box<dyn Fn(&mut DrawingContextImpl)>;

fn sequence() -> Vec<Draw> {
    let fill = |brush: Rc<dyn IBrush>, x: f64| -> Draw {
        Box::new(move |context| {
            let rect = RoundedRect::from(Rect::new(x, 4.0, 14.0, 14.0));
            context.draw_rectangle(Some(&*brush), None, rect, &BoxShadows::default());
        })
    };
    let line = |pen: Rc<dyn IPen>, y: f64| -> Draw {
        Box::new(move |context| context.draw_line(Some(&*pen), Point::new(6.0, y), Point::new(90.0, y + 7.0)))
    };
    let outline = |pen: Rc<dyn IPen>, x: f64| -> Draw {
        Box::new(move |context| {
            let rect = RoundedRect::from(Rect::new(x, 60.0, 16.0, 16.0));
            context.draw_rectangle(None, Some(&*pen), rect, &BoxShadows::default());
        })
    };
    let aliased: Draw = Box::new(|context| {
        context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() })
    });
    let pop_options: Draw = Box::new(|context| context.pop_render_options());
    let opacity: Draw = Box::new(|context| context.push_opacity(0.5, None));
    let pop_opacity: Draw = Box::new(|context| context.pop_opacity());
    vec![
        fill(solid(Colors::RED), 2.0),
        fill(gradient(), 18.0),
        fill(solid(Colors::RED), 34.0),
        fill(solid(Colors::GREEN), 50.0),
        aliased,
        fill(solid(Colors::GREEN), 66.0),
        line(pen(Colors::BLUE, 2.0, false, PenLineCap::Flat, PenLineJoin::Miter), 24.0),
        pop_options,
        line(pen(Colors::BLUE, 2.0, false, PenLineCap::Flat, PenLineJoin::Miter), 32.0),
        line(pen(Colors::BLUE, 3.0, true, PenLineCap::Flat, PenLineJoin::Miter), 40.0),
        line(pen(Colors::BLUE, 3.0, false, PenLineCap::Round, PenLineJoin::Miter), 48.0),
        outline(pen(Colors::BLACK, 2.0, false, PenLineCap::Flat, PenLineJoin::Round), 4.0),
        outline(pen(Colors::BLACK, 2.0, false, PenLineCap::Flat, PenLineJoin::Bevel), 24.0),
        opacity,
        outline(pen(Colors::BLACK, 2.0, false, PenLineCap::Flat, PenLineJoin::Bevel), 44.0),
        fill(solid(Colors::RED), 82.0),
        pop_opacity,
        outline(pen(Colors::RED, 4.0, true, PenLineCap::Square, PenLineJoin::Bevel), 64.0),
        outline(pen(Colors::RED, 4.0, false, PenLineCap::Square, PenLineJoin::Bevel), 80.0),
        fill(gradient(), 66.0),
        fill(solid(Colors::BLUE), 2.0),
    ]
}

fn white() -> Surface {
    let mut surface = surfaces::raster_n32_premul((100, 100)).expect("a raster surface");
    surface.canvas().clear(skia_safe::Color::WHITE);
    surface
}

fn pixels(surface: &mut Surface) -> Vec<u8> {
    surface.peek_pixels().expect("raster pixels").bytes().expect("the bytes").to_vec()
}

#[test]
fn kept_paints_draw_what_fresh_paints_draw_through_solid_gradient_dashed_and_option_changes() {
    // One context for the whole sequence: the paints are kept from draw to draw.
    let mut kept = white();
    let mut context = wrap_skia_surface(&kept, Vector::new(96.0, 96.0));
    for draw in sequence() {
        draw(&mut context);
    }
    context.dispose();

    // The same sequence with paints no draw before has used: the kept paints are replaced by new ones before
    // every draw.
    let mut fresh = white();
    let mut context = wrap_skia_surface(&fresh, Vector::new(96.0, 96.0));
    for draw in sequence() {
        context.reset_kept_paints_for_tests();
        draw(&mut context);
    }
    context.dispose();
    assert!(pixels(&kept) == pixels(&fresh));
}
