//! Port of upstream's `Shapes/PathTests.cs`.
//!
//! `GetWidenedPathGeometry_Line` passes `gpuAllowedError: 0.05` to
//! `CompareImages` upstream. That argument concerns the Mesa GL outputs of
//! upstream, which the port does not render, so it is dropped.

use crate::test_base::TestBase;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{
    Brushes, DashStyle, IPen, LineSegment, MediaCollection, PathFigure, PathFigures, PathGeometry, PathSegments, Pen,
    PenLineCap, PenLineJoin, RectangleGeometry, RotateTransform, Stretch, StreamGeometry,
};
use ferroui_base::{Point, Rect, Thickness};
use ferroui_controls::shapes::Path;
use ferroui_controls::{Border, Decorator};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Shapes\Path")
}

#[test]
fn line_absolute() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M 10,190 L 190,10 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Line_Absolute");
    t.compare_images("Line_Absolute");
}

#[test]
fn line_relative() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M10,190 l190,-190 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Line_Relative");
    t.compare_images("Line_Relative");
}

#[test]
fn horizontal_line_absolute() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M190,100 H10 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "HorizontalLine_Absolute");
    t.compare_images("HorizontalLine_Absolute");
}

#[test]
fn horizontal_line_relative() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M190,100 h-180 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "HorizontalLine_Relative");
    t.compare_images("HorizontalLine_Relative");
}

#[test]
fn vertical_line_absolute() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M100,190 V10 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "VerticalLine_Absolute");
    t.compare_images("VerticalLine_Absolute");
}

#[test]
fn vertical_line_relative() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M100,190 V-180 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "VerticalLine_Relative");
    t.compare_images("VerticalLine_Relative");
}

#[test]
fn cubic_bezier_absolute() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M190,0 C10,10 190,190 10,190 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "CubicBezier_Absolute");
    t.compare_images("CubicBezier_Absolute");
}

#[test]
fn cubic_bezier_relative() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M190,0 c-180,10 0,190 -180,190 M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "CubicBezier_Relative");
    t.compare_images("CubicBezier_Relative");
}

#[test]
fn arc_absolute() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M190,100 A90,90 0 1,0 10,100  M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Arc_Absolute");
    t.compare_images("Arc_Absolute");
}

#[test]
fn arc_relative() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M190,100 a90,90 0 1,0 -180,0  M0,0M200,200").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Arc_Relative");
    t.compare_images("Arc_Relative");
}

#[test]
fn path_100px_triangle_centered() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(2.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M 0,100 L 100,100 50,0 Z").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Path_100px_Triangle_Centered");
    t.compare_images("Path_100px_Triangle_Centered");
}

#[test]
fn path_tick_scaled() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(2.0);
    child.set_stretch(Stretch::Uniform);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M 1145.607177734375,430 C1145.607177734375,430 1141.449951171875,435.0772705078125 1141.449951171875,435.0772705078125 1141.449951171875,435.0772705078125 1139.232177734375,433.0999755859375 1139.232177734375,433.0999755859375 1139.232177734375,433.0999755859375 1138,434.5538330078125 1138,434.5538330078125 1138,434.5538330078125 1141.482177734375,438 1141.482177734375,438 1141.482177734375,438 1141.96875,437.9375 1141.96875,437.9375 1141.96875,437.9375 1147,431.34619140625 1147,431.34619140625 1147,431.34619140625 1145.607177734375,430 1145.607177734375,430 z").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Path_Tick_Scaled");
    t.compare_images("Path_Tick_Scaled");
}

#[test]
fn path_tick_scaled_stroke_8px() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_fill(Some(Brushes::gray()));
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(8.0);
    child.set_stretch(Stretch::Uniform);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(StreamGeometry::parse("M 1145.607177734375,430 C1145.607177734375,430 1141.449951171875,435.0772705078125 1141.449951171875,435.0772705078125 1141.449951171875,435.0772705078125 1139.232177734375,433.0999755859375 1139.232177734375,433.0999755859375 1139.232177734375,433.0999755859375 1138,434.5538330078125 1138,434.5538330078125 1138,434.5538330078125 1141.482177734375,438 1141.482177734375,438 1141.482177734375,438 1141.96875,437.9375 1141.96875,437.9375 1141.96875,437.9375 1147,431.34619140625 1147,431.34619140625 1147,431.34619140625 1145.607177734375,430 1145.607177734375,430 z").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Path_Tick_Scaled_Stroke_8px");
    t.compare_images("Path_Tick_Scaled_Stroke_8px");
}

#[test]
fn path_expander_with_border() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let border = Border::new();
    border.set_border_brush(Some(Brushes::red()));
    border.set_border_thickness(Thickness::uniform(1.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    let path = Path::new();
    path.set_fill(Some(Brushes::black()));
    path.set_stroke(Some(Brushes::black()));
    path.set_stroke_thickness(1.0);
    path.set_stretch(Stretch::Uniform);
    path.set_data(StreamGeometry::parse("M 0 2 L 4 6 L 0 10 Z").expect("the path data is valid"));
    border.set_child(path);
    target.set_child(border);

    t.render_to_file(&target, "Path_Expander_With_Border");
    t.compare_images("Path_Expander_With_Border");
}

#[test]
fn path_with_pen_line_cap() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(10.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_stroke_dash_array(Some(MediaCollection::from_items([3.0, 1.0])));
    child.set_stroke_line_cap(PenLineCap::Round);
    child.set_data(StreamGeometry::parse("M 20,20 L 180,180").expect("the path data is valid"));
    target.set_child(child);

    t.render_to_file(&target, "Path_With_PenLineCap");
    t.compare_images("Path_With_PenLineCap");
}

#[test]
fn path_with_rotated_geometry() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let child = Path::new();
    child.set_fill(Some(Brushes::red()));
    let data = RectangleGeometry::new();
    data.set_rect(Rect::new(50.0, 50.0, 100.0, 100.0));
    data.set_transform(RotateTransform::with_angle(45.0));
    child.set_data(data);
    target.set_child(child);

    t.render_to_file(&target, "Path_With_Rotated_Geometry");
    t.compare_images("Path_With_Rotated_Geometry");
}

#[test]
fn begin_figure_is_filled_is_respected() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    let child = Path::new();
    child.set_fill(Some(Brushes::black()));
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(10.0);
    let data = PathGeometry::new();
    let figures = PathFigures::new();

    let figure = PathFigure::new();
    figure.set_is_filled(false);
    figure.set_is_closed(false);
    figure.set_start_point(Point::new(170.0, 170.0));
    let segments = PathSegments::new();
    let segment = LineSegment::new();
    segment.set_point(Point::new(60.0, 170.0));
    segments.add(segment.upcast());
    let segment = LineSegment::new();
    segment.set_point(Point::new(60.0, 60.0));
    segments.add(segment.upcast());
    figure.set_segments(Some(segments));
    figures.add(figure);

    let figure = PathFigure::new();
    figure.set_is_filled(true);
    figure.set_is_closed(true);
    figure.set_start_point(Point::new(60.0, 20.0));
    let segments = PathSegments::new();
    let segment = LineSegment::new();
    segment.set_point(Point::new(20.0, 60.0));
    segments.add(segment.upcast());
    let segment = LineSegment::new();
    segment.set_point(Point::new(100.0, 60.0));
    segments.add(segment.upcast());
    figure.set_segments(Some(segments));
    figures.add(figure);

    data.set_figures(Some(figures));
    child.set_data(data);
    target.set_child(child);

    t.render_to_file(&target, "BeginFigure_IsFilled_Is_Respected");
    t.compare_images("BeginFigure_IsFilled_Is_Respected");
}

#[test]
fn get_widened_path_geometry_line() {
    let t = base();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 10.0).into();
    let geometry = StreamGeometry::parse("M 0,0 L 180,180").expect("the path data is valid").get_widened_geometry(&*pen);

    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_fill(Some(Brushes::green()));
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(geometry);
    target.set_child(child);

    t.render_to_file(&target, "GetWidenedPathGeometry_Line");
    t.compare_images("GetWidenedPathGeometry_Line");
}

#[test]
fn get_widened_path_geometry_line_dash() {
    let t = base();
    // The constructor of upstream's pen has these defaults after the dash style: a flat cap, a miter join and
    // a miter limit of 10.
    let pen: Rc<dyn IPen> =
        Pen::with_all(Some(Brushes::black()), 10.0, Some(DashStyle::dash()), PenLineCap::Flat, PenLineJoin::Miter, 10.0)
            .into();
    let geometry = StreamGeometry::parse("M 0,0 L 180,180").expect("the path data is valid").get_widened_geometry(&*pen);

    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Path::new();
    child.set_stroke(Some(Brushes::red()));
    child.set_stroke_thickness(1.0);
    child.set_fill(Some(Brushes::green()));
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    child.set_data(geometry);
    target.set_child(child);

    t.render_to_file(&target, "GetWidenedPathGeometry_Line_Dash");
    t.compare_images("GetWidenedPathGeometry_Line_Dash");
}
