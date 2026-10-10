//! Port of upstream's `CrossTests/CrossGeometryTests.cs`.
//!
//! `Should_Properly_CloseFigure` is compiled upstream only outside the Skia
//! configuration and is not ported.

use crate::cross_test_base::CrossTestBase;
use crate::cross_ui::*;
use ferroui_base::media::{Colors, TileMode};
use ferroui_base::{Point, Rect};

fn base() -> CrossTestBase {
    CrossTestBase::new("Media/Geometry")
}

fn white_rectangle_and_outline(geometry: CrossStreamGeometry) -> CrossDrawingBrush {
    let mut brush = CrossDrawingBrush::new(CrossDrawingGroup {
        children: vec![
            {
                let mut drawing =
                    CrossGeometryDrawing::new(CrossRectangleGeometry::new(Rect::new(0.0, 0.0, 300.0, 280.0)));
                drawing.brush = CrossSolidColorBrush::new(Colors::WHITE).into();
                drawing.into()
            },
            {
                let mut drawing = CrossGeometryDrawing::new(geometry);
                drawing.pen = Some({
                    let mut pen = CrossPen::new(CrossSolidColorBrush::new(Colors::BLACK));
                    pen.thickness = 2.0;
                    pen
                });
                drawing.into()
            },
        ],
    });
    brush.tile_mode = TileMode::None;
    brush
}

#[test]
fn should_render_stream_geometry() {
    let t = base();
    let geometry = CrossStreamGeometry::new();

    let context = geometry.get_context();
    context.begin_figure(Point::new(150.0, 15.0), true, true);
    context.line_to(Point::new(258.0, 77.0), true);
    context.line_to(Point::new(258.0, 202.0), true);
    context.line_to(Point::new(150.0, 265.0), true);
    context.line_to(Point::new(42.0, 202.0), true);
    context.line_to(Point::new(42.0, 77.0), true);
    context.end_figure();

    // The brush of the two tests of stream geometries is the same upstream, written out in each.
    let brush = white_rectangle_and_outline(geometry);

    let mut root = CrossControl::new();
    root.width = 300.0;
    root.height = 280.0;
    root.background = brush.into();
    t.render_and_compare(root, "Should_Render_Stream_Geometry");
}

#[test]
fn should_render_geometry_with_strokeless_lines() {
    let t = base();
    let geometry = CrossStreamGeometry::new();

    let context = geometry.get_context();
    context.begin_figure(Point::new(150.0, 15.0), true, true);
    context.line_to(Point::new(258.0, 77.0), true);
    context.line_to(Point::new(258.0, 202.0), false);
    context.line_to(Point::new(150.0, 265.0), true);
    context.line_to(Point::new(42.0, 202.0), true);
    context.line_to(Point::new(42.0, 77.0), false);
    context.end_figure();

    let brush = white_rectangle_and_outline(geometry);

    let mut root = CrossControl::new();
    root.width = 300.0;
    root.height = 280.0;
    root.background = brush.into();
    t.render_and_compare(root, "Should_Render_Geometry_With_Strokeless_Lines");
}

#[test]
fn should_render_poly_line_segment_with_strokeless_lines() {
    let t = base();
    let brush: CrossBrush = CrossSolidColorBrush::new(Colors::BLUE).into();
    let pen = {
        let mut pen = CrossPen::new(CrossSolidColorBrush::new(Colors::RED));
        pen.thickness = 8.0;
        pen
    };
    let figure = CrossPathFigure {
        closed: true,
        segments: vec![CrossPathSegment::PolyLine {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(100.0, 100.0),
                Point::new(0.0, 100.0),
                Point::new(0.0, 0.0),
            ],
            is_stroked: false,
        }],
        ..Default::default()
    };
    let geometry: CrossGeometry = CrossPathGeometry { figures: vec![figure] }.into();

    let mut control = CrossFuncControl::new(move |ctx| ctx.draw_geometry(Some(&brush), Some(&pen), &geometry));
    control.width = 100.0;
    control.height = 100.0;

    t.render_and_compare(control, "Should_Render_PolyLineSegment_With_Strokeless_Lines");
}

#[test]
fn should_render_poly_bezier_segment_with_strokeless_lines() {
    let t = base();
    let brush: CrossBrush = CrossSolidColorBrush::new(Colors::BLUE).into();
    let pen = {
        let mut pen = CrossPen::new(CrossSolidColorBrush::new(Colors::RED));
        pen.thickness = 8.0;
        pen
    };
    let figure = CrossPathFigure {
        start: Point::new(10.0, 100.0),
        closed: false,
        segments: vec![CrossPathSegment::PolyBezierSegment {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(200.0, 0.0),
                Point::new(300.0, 100.0),
                Point::new(300.0, 0.0),
                Point::new(500.0, 0.0),
                Point::new(600.0, 100.0),
            ],
            is_stroked: false,
        }],
    };
    let geometry: CrossGeometry = CrossPathGeometry { figures: vec![figure] }.into();

    let mut control = CrossFuncControl::new(move |ctx| ctx.draw_geometry(Some(&brush), Some(&pen), &geometry));
    control.width = 700.0;
    control.height = 400.0;

    t.render_and_compare(control, "Should_Render_PolyBezierSegment_With_Strokeless_Lines");
}
