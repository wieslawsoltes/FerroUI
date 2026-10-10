//! Port of upstream's `Media/TileBrushTests.cs`, whose class is
//! `DrawingBrushTests`.

use crate::test_base::TestBase;
use ferroui_base::media::{
    Brushes, Colors, DrawingBrush, Geometry, GeometryDrawing, GeometryGroup, GradientStop, LinearGradientBrush, Pen,
    RectangleGeometry, Stretch, TileMode,
};
use ferroui_base::{Rect, Ref, RelativeRect, RelativeUnit, Thickness};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::Decorator;

fn base() -> TestBase {
    TestBase::new(r"Media\DrawingBrush")
}

#[test]
fn drawing_brush_is_properly_tiled() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_width(220.0);
    target.set_height(220.0);
    let child = Rectangle::new();
    let fill = DrawingBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::Tile);
    fill.set_drawing(create_drawing());
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.25, 0.25, RelativeUnit::Relative));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "DrawingBrushIsProperlyTiled");
    t.compare_images("DrawingBrushIsProperlyTiled");
}

#[test]
fn drawing_brush_is_properly_scaled() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_width(220.0);
    target.set_height(220.0);
    let child = Rectangle::new();
    let fill = DrawingBrush::new();
    fill.set_tile_mode(TileMode::Tile);
    fill.set_source_rect(RelativeRect::new(0.0, 0.0, 20.0, 20.0, RelativeUnit::Absolute));
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 20.0, 20.0, RelativeUnit::Absolute));
    let drawing = GeometryDrawing::new();
    drawing.set_pen(Some(Pen::with_brush(Some(Brushes::red()), 5.0).into()));
    drawing.set_geometry(Geometry::parse("M 0 0 l 50 50").unwrap());
    fill.set_drawing(drawing);
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "DrawingBrushIsProperlyScaled");
    t.compare_images("DrawingBrushIsProperlyScaled");
}

#[test]
fn drawing_brush_is_properly_upscaled() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(10.0));
    target.set_width(420.0);
    target.set_height(420.0);
    let child = Rectangle::new();
    let fill = DrawingBrush::new();
    fill.set_stretch(Stretch::Fill);
    fill.set_tile_mode(TileMode::None);
    fill.set_drawing(create_drawing());
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "DrawingBrushIsProperlyUpscaled");
    t.compare_images("DrawingBrushIsProperlyUpscaled");
}

fn create_drawing() -> Ref<GeometryDrawing> {
    let drawing = GeometryDrawing::new();
    let geometry = GeometryGroup::new();
    geometry.children().add(RectangleGeometry::with_rect(Rect::new(50.0, 25.0, 25.0, 25.0)).upcast());
    geometry.children().add(RectangleGeometry::with_rect(Rect::new(25.0, 50.0, 25.0, 25.0)).upcast());
    drawing.set_geometry(geometry);
    let brush = LinearGradientBrush::new();
    brush.gradient_stops().add(GradientStop::with_color_and_offset(Colors::BLUE, 0.0));
    brush.gradient_stops().add(GradientStop::with_color_and_offset(Colors::BLACK, 1.0));
    drawing.set_pen(Some(Pen::with_brush(Some(brush.into()), 5.0).into()));
    drawing.set_brush(Some(Brushes::yellow()));
    drawing
}
