//! Port of upstream's `Media/StreamGeometryTests.cs`.
//!
//! Upstream's test renders its two outputs and compares neither with an
//! expected image; so does the port.

use crate::test_base::TestBase;
use ferroui_base::media::{Colors, SolidColorBrush, StreamGeometry, SweepDirection};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Size, Thickness};
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::shapes::Path;

fn base() -> TestBase {
    TestBase::new(r"Media\StreamGeometry")
}

#[test]
fn precise_elliptic_arc_produces_valid_arcs_in_all_directions() {
    let t = base();
    let grid = UniformGrid::new();
    grid.set_columns(2);
    grid.set_rows(4);
    grid.set_width(320.0);
    grid.set_height(400.0);
    for sweep_direction in [SweepDirection::Clockwise, SweepDirection::CounterClockwise] {
        for is_large_arc in [false, true] {
            for is_precise in [false, true] {
                let pt = |x: f64, y: f64| Point::new(x, y);
                let sz = |w: f64, h: f64| Size::new(w, h);
                let stream_geometry = StreamGeometry::new();
                {
                    let mut context = stream_geometry.open();
                    context.begin_figure(pt(20.0, 20.0), true);

                    if is_precise {
                        context.precise_arc_to(pt(40.0, 40.0), sz(20.0, 20.0), 0.0, is_large_arc, sweep_direction);
                    } else {
                        context.arc_to(pt(40.0, 40.0), sz(20.0, 20.0), 0.0, is_large_arc, sweep_direction, true);
                    }
                    context.line_to(pt(40.0, 20.0), true);
                    context.line_to(pt(20.0, 20.0), true);
                    context.end_figure(true);
                }
                let path_shape = Path::new();
                path_shape.set_data(stream_geometry);
                path_shape.set_stroke(Some(SolidColorBrush::with_color(Colors::CORNFLOWER_BLUE).into()));
                path_shape.set_fill(Some(SolidColorBrush::with_color(Colors::GOLD).into()));
                path_shape.set_stroke_thickness(2.0);
                path_shape.set_margin(Thickness::uniform(20.0));
                grid.children().add(path_shape);
            }
        }
    }
    t.render_to_file(&grid, "PreciseEllipticArc_Produces_Valid_Arcs_In_All_Directions");
}
