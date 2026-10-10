//! Port of upstream's `CrossTests/Brushes/CrossTileBrushTests.cs`.
//!
//! Upstream finds `Ramp64.png` next to the test assembly; here it is in the
//! `Assets` directory of the test crate.

use crate::cross_test_base::CrossTestBase;
use crate::cross_ui::*;
use crate::test_render_helper;
use ferroui_base::media::{AlignmentX, AlignmentY, BrushMappingMode, Colors, Stretch, TileMode};
use ferroui_base::{Matrix, Rect};

fn base() -> CrossTestBase {
    CrossTestBase::new("Media/TileBrushes")
}

fn geometry_drawing_with_brush(rect: Rect, brush: CrossSolidColorBrush) -> CrossDrawing {
    let mut drawing = CrossGeometryDrawing::new(CrossRectangleGeometry::new(rect));
    drawing.brush = brush.into();
    drawing.into()
}

#[test]
fn simple_checkboard_pattern_is_rendered_identically() {
    let t = base();
    let mut root = CrossControl::new();
    root.width = 100.0;
    root.height = 100.0;
    root.background = {
        let mut brush = CrossDrawingBrush::new({
            let mut group = CrossDrawingGroup::default();
            group.children.push(geometry_drawing_with_brush(
                Rect::new(0.0, 0.0, 20.0, 20.0),
                CrossSolidColorBrush::new(Colors::WHITE),
            ));
            group.children.push(geometry_drawing_with_brush(
                Rect::new(0.0, 0.0, 10.0, 10.0),
                CrossSolidColorBrush::new(Colors::BLACK),
            ));
            group.children.push(geometry_drawing_with_brush(
                Rect::new(10.0, 10.0, 10.0, 10.0),
                CrossSolidColorBrush::new(Colors::BLACK),
            ));
            group
        });
        brush.viewport = Rect::new(0.0, 0.0, 10.0, 10.0);
        brush.viewport_units = BrushMappingMode::Absolute;
        brush.tile_mode = TileMode::Tile;
        brush.into()
    };
    t.render_and_compare(root, "Simple_Checkboard_Pattern_Is_Rendered_Identically");
}

#[test]
fn should_render_scaled_tile_brush() {
    let t = base();
    let mut brush = CrossDrawingBrush::new({
        let mut drawing = CrossGeometryDrawing::new(CrossSvgGeometry::new("M 0 0 l 50 50"));
        drawing.pen = Some({
            let mut pen = CrossPen::new(CrossSolidColorBrush::new(Colors::RED));
            pen.thickness = 5.0;
            pen
        });
        drawing
    });
    brush.tile_mode = TileMode::Tile;
    brush.viewbox = Rect::new(0.0, 0.0, 20.0, 20.0);
    brush.viewbox_units = BrushMappingMode::Absolute;
    brush.viewport = Rect::new(0.0, 0.0, 20.0, 20.0);
    brush.viewport_units = BrushMappingMode::Absolute;

    let mut root = CrossControl::new();
    root.width = 100.0;
    root.height = 100.0;
    root.background = brush.into();
    t.render_and_compare(root, "Should_Render_Scaled_TileBrush");
}

#[test]
fn should_render_aligned_tile_brush() {
    let t = base();
    let mut brush = CrossDrawingBrush::new(CrossDrawingGroup {
        children: vec![geometry_drawing_with_brush(
            Rect::new(0.0, 0.0, 100.0, 150.0),
            CrossSolidColorBrush::new(Colors::CRIMSON),
        )],
    });
    brush.tile_mode = TileMode::Tile;
    brush.alignment_x = AlignmentX::Center;
    brush.alignment_y = AlignmentY::Center;
    brush.stretch = Stretch::Uniform;

    let mut root = CrossControl::new();
    root.width = 100.0;
    root.height = 100.0;
    root.background = brush.into();
    t.render_and_compare(root, "Should_Render_Aligned_TileBrush");
}

#[test]
fn should_render_tile_brush_with_tile_mode_none() {
    let t = base();
    let mut brush = CrossDrawingBrush::new(CrossDrawingGroup {
        children: vec![geometry_drawing_with_brush(
            Rect::new(0.0, 0.0, 50.0, 50.0),
            CrossSolidColorBrush::new(Colors::CRIMSON),
        )],
    });
    brush.tile_mode = TileMode::None;
    brush.stretch = Stretch::Fill;
    brush.viewbox = Rect::new(0.0, 0.0, 50.0, 50.0);
    brush.viewbox_units = BrushMappingMode::Absolute;
    brush.viewport = Rect::new(0.0, 0.0, 50.0, 50.0);
    brush.viewport_units = BrushMappingMode::Absolute;

    let mut root = CrossControl::new();
    root.width = 200.0;
    root.height = 200.0;
    root.background = brush.into();
    t.render_and_compare(root, "Should_Render_TileBrush_With_TileMode_None");
}

// Scaling about the centre of the 200x200 canvas rather than about its origin, so the
// transformed tile stays inside the fill and the output shows where it landed.
fn halve_about_canvas_centre() -> Matrix {
    Matrix::create_translation(-100.0, -100.0) * Matrix::create_scale(0.5, 0.5) * Matrix::create_translation(100.0, 100.0)
}

#[test]
fn should_render_drawing_brush_with_transform_on_an_offset_fill() {
    let t = base();
    // A relative viewport lays the tile over the fill, and the transform then acts on it in
    // target space, so the crimson rect ends up centred in the fill at half its size.
    // Transforming the tile before it is positioned would push it towards the bottom edge.
    let mut brush = CrossDrawingBrush::new(geometry_drawing_with_brush(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        CrossSolidColorBrush::new(Colors::CRIMSON),
    ));
    brush.tile_mode = TileMode::None;
    brush.transform = Some(halve_about_canvas_centre());
    let brush: CrossBrush = brush.into();

    let mut root =
        CrossFuncControl::new(move |ctx| ctx.draw_rectangle(Some(&brush), None, Rect::new(40.0, 70.0, 120.0, 60.0)));
    root.width = 200.0;
    root.height = 200.0;
    root.background = CrossSolidColorBrush::new(Colors::WHITE).into();
    t.render_and_compare(root, "Should_Render_Drawing_Brush_With_Transform_On_An_Offset_Fill");
}

#[test]
fn should_render_image_brush_with_transform_on_an_offset_fill() {
    let t = base();
    let path = test_render_helper::get_tests_directory().join("Assets").join("Ramp64.png");

    let mut brush = CrossImageBrush::new(path.to_str().expect("the path is text"));
    brush.transform = Some(halve_about_canvas_centre());
    let brush: CrossBrush = brush.into();

    let mut root =
        CrossFuncControl::new(move |ctx| ctx.draw_rectangle(Some(&brush), None, Rect::new(40.0, 70.0, 120.0, 60.0)));
    root.width = 200.0;
    root.height = 200.0;
    root.background = CrossSolidColorBrush::new(Colors::WHITE).into();
    t.render_and_compare(root, "Should_Render_Image_Brush_With_Transform_On_An_Offset_Fill");
}

#[test]
fn should_render_with_transform() {
    let t = base();
    let mut brush = CrossDrawingBrush::new(CrossDrawingGroup {
        children: vec![
            geometry_drawing_with_brush(Rect::new(0.0, 0.0, 100.0, 100.0), CrossSolidColorBrush::new(Colors::CRIMSON)),
            geometry_drawing_with_brush(Rect::new(20.0, 20.0, 60.0, 60.0), CrossSolidColorBrush::new(Colors::BLUE)),
        ],
    });
    brush.tile_mode = TileMode::None;
    brush.viewbox = Rect::new(0.0, 0.0, 1.0, 1.0);
    brush.viewbox_units = BrushMappingMode::RelativeToBoundingBox;
    brush.viewport = Rect::new(0.0, 0.0, 50.0, 50.0);
    brush.viewport_units = BrushMappingMode::Absolute;
    brush.transform = Some(Matrix::create_translation(150.0, 150.0));

    let mut root = CrossControl::new();
    root.width = 200.0;
    root.height = 200.0;
    root.background = brush.into();
    t.render_and_compare(root, "Should_Render_With_Transform");
}
