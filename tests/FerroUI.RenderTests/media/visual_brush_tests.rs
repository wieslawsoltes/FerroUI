//! Port of upstream's `Media/VisualBrushTests.cs`.
//!
//! Upstream loads the source image from its output directory, which is
//! also the directory of its expected images; the port loads it from the
//! directory of the expected images.
//!
//! `VisualBrush_Grip_144_Dpi` passes `gpuAllowedError: 0.05` upstream,
//! which concerns upstream's Mesa GL outputs; the port does not render
//! them and the argument is dropped.

use crate::media::relative_point_test_primitives_helper::RelativePointTestPrimitivesHelper;
use crate::test_base::TestBase;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{AlignmentX, AlignmentY, Brushes, Stretch, TileMode, VisualBrush};
use ferroui_base::{Ref, RelativeRect, RelativeUnit, Thickness};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, Canvas, ColumnDefinitions, Control, Decorator, Grid, Image, Panel, RowDefinitions};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\VisualBrush")
}

fn bitmap_path(t: &TestBase) -> String {
    t.expected_path().join("github_icon.png").to_str().expect("the path is text").to_string()
}

/// Upstream's `Visual` property: a new tree at every read.
fn visual(t: &TestBase) -> Ref<Control> {
    let panel = Panel::new();
    let image = Image::new();
    image.set_source(Some(Rc::new(Bitmap::from_file(&bitmap_path(t)).expect("the image is loaded"))));
    panel.children().add(image);
    let border = Border::new();
    border.set_border_brush(Some(Brushes::blue()));
    border.set_border_thickness(Thickness::uniform(2.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    let child = Panel::new();
    child.set_height(26.0);
    child.set_width(150.0);
    child.set_background(Some(Brushes::green()));
    border.set_child(child);
    panel.children().add(border);
    panel.upcast()
}

#[test]
fn visual_brush_no_stretch_no_tile_alignment_top_left() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_alignment_x(AlignmentX::Left);
    fill.set_alignment_y(AlignmentY::Top);
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_NoTile_Alignment_TopLeft");
    t.compare_images("VisualBrush_NoStretch_NoTile_Alignment_TopLeft");
}

#[test]
fn visual_brush_no_stretch_no_tile_alignment_center() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_alignment_x(AlignmentX::Center);
    fill.set_alignment_y(AlignmentY::Center);
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_NoTile_Alignment_Center");
    t.compare_images("VisualBrush_NoStretch_NoTile_Alignment_Center");
}

#[test]
fn visual_brush_no_stretch_no_tile_alignment_bottom_right() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_alignment_x(AlignmentX::Right);
    fill.set_alignment_y(AlignmentY::Bottom);
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_NoTile_Alignment_BottomRight");
    t.compare_images("VisualBrush_NoStretch_NoTile_Alignment_BottomRight");
}

#[test]
fn visual_brush_fill_no_tile() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(920.0);
    target.set_height(920.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::Fill);
    fill.set_tile_mode(TileMode::None);
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_Fill_NoTile");
    t.compare_images("VisualBrush_Fill_NoTile");
}

#[test]
fn visual_brush_uniform_no_tile() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(300.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::Uniform);
    fill.set_tile_mode(TileMode::None);
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_Uniform_NoTile");
    t.compare_images("VisualBrush_Uniform_NoTile");
}

#[test]
fn visual_brush_uniform_to_fill_no_tile() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(300.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::UniformToFill);
    fill.set_tile_mode(TileMode::None);
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_UniformToFill_NoTile");
    t.compare_images("VisualBrush_UniformToFill_NoTile");
}

#[test]
fn visual_brush_no_stretch_no_tile_bottom_right_quarter_source() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_source_rect(RelativeRect::new(250.0, 250.0, 250.0, 250.0, RelativeUnit::Absolute));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_NoTile_BottomRightQuarterSource");
    t.compare_images("VisualBrush_NoStretch_NoTile_BottomRightQuarterSource");
}

#[test]
fn visual_brush_no_stretch_no_tile_bottom_right_quarter_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_destination_rect(RelativeRect::new(92.0, 92.0, 92.0, 92.0, RelativeUnit::Absolute));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_NoTile_BottomRightQuarterDest");
    t.compare_images("VisualBrush_NoStretch_NoTile_BottomRightQuarterDest");
}

#[test]
fn visual_brush_no_stretch_no_tile_bottom_right_quarter_source_bottom_right_quarter_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_source_rect(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_destination_rect(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_NoTile_BottomRightQuarterSource_BottomRightQuarterDest");
    t.compare_images("VisualBrush_NoStretch_NoTile_BottomRightQuarterSource_BottomRightQuarterDest");
}

#[test]
fn visual_brush_no_stretch_tile_bottom_right_quarter_source_center_quarter_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::Tile);
    fill.set_source_rect(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_destination_rect(RelativeRect::new(0.25, 0.25, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_Tile_BottomRightQuarterSource_CenterQuarterDest");
    t.compare_images("VisualBrush_NoStretch_Tile_BottomRightQuarterSource_CenterQuarterDest");
}

#[test]
fn visual_brush_no_stretch_flip_x_top_left_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::FlipX);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_FlipX_TopLeftDest");
    t.compare_images("VisualBrush_NoStretch_FlipX_TopLeftDest");
}

#[test]
fn visual_brush_no_stretch_flip_y_top_left_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::FlipY);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_FlipY_TopLeftDest");
    t.compare_images("VisualBrush_NoStretch_FlipY_TopLeftDest");
}

#[test]
fn visual_brush_no_stretch_flip_xy_top_left_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = VisualBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::FlipXY);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_visual(visual(&t));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "VisualBrush_NoStretch_FlipXY_TopLeftDest");
    t.compare_images("VisualBrush_NoStretch_FlipXY_TopLeftDest");
}

#[test]
fn visual_brush_in_tree_visual() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse("Auto,*").unwrap());
    let source = Border::new();
    source.set_background(Some(Brushes::yellow()));
    source.set_horizontal_alignment(HorizontalAlignment::Left);
    let panel = Panel::new();
    panel.set_height(10.0);
    panel.set_width(50.0);
    source.set_child(panel);
    grid.children().add(&source);
    let border = Border::new();
    let background = VisualBrush::new();
    background.set_stretch(Stretch::Uniform);
    background.set_visual(&source);
    border.set_background(Some(background.into()));
    Grid::set_row(&border, 1);
    grid.children().add(border);
    target.set_child(grid);

    t.render_to_file(&target, "VisualBrush_InTree_Visual");
    t.compare_images("VisualBrush_InTree_Visual");
}

#[test]
fn visual_brush_grip_96_dpi() {
    let t = base();
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(10.0);
    let background = VisualBrush::new();
    background.set_source_rect(RelativeRect::new(0.0, 0.0, 4.0, 5.0, RelativeUnit::Absolute));
    background.set_destination_rect(RelativeRect::new(0.0, 0.0, 4.0, 5.0, RelativeUnit::Absolute));
    background.set_tile_mode(TileMode::Tile);
    background.set_stretch(Stretch::UniformToFill);
    let canvas = Canvas::new();
    canvas.set_width(4.0);
    canvas.set_height(5.0);
    canvas.set_background(Some(Brushes::white_smoke()));
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_left(&rectangle, 2.0);
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_top(&rectangle, 2.0);
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_left(&rectangle, 2.0);
    Canvas::set_top(&rectangle, 4.0);
    canvas.children().add(rectangle);
    background.set_visual(canvas);
    target.set_background(Some(background.into()));

    t.render_to_file(&target, "VisualBrush_Grip_96_Dpi");
    t.compare_images("VisualBrush_Grip_96_Dpi");
}

#[test]
fn visual_brush_grip_144_dpi() {
    let t = base();
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(7.5);
    let background = VisualBrush::new();
    background.set_source_rect(RelativeRect::new(0.0, 0.0, 4.0, 5.0, RelativeUnit::Absolute));
    background.set_destination_rect(RelativeRect::new(0.0, 0.0, 4.0, 5.0, RelativeUnit::Absolute));
    background.set_tile_mode(TileMode::Tile);
    background.set_stretch(Stretch::UniformToFill);
    let canvas = Canvas::new();
    canvas.set_width(4.0);
    canvas.set_height(5.0);
    canvas.set_background(Some(Brushes::white_smoke()));
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_left(&rectangle, 2.0);
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_top(&rectangle, 2.0);
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_left(&rectangle, 2.0);
    Canvas::set_top(&rectangle, 4.0);
    canvas.children().add(rectangle);
    background.set_visual(canvas);
    target.set_background(Some(background.into()));

    t.render_to_file_with_dpi(&target, "VisualBrush_Grip_144_Dpi", 144.0);
    t.compare_images("VisualBrush_Grip_144_Dpi");
}

#[test]
fn visual_brush_grip_192_dpi() {
    let t = base();
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(10.0);
    let background = VisualBrush::new();
    background.set_source_rect(RelativeRect::new(0.0, 0.0, 4.0, 5.0, RelativeUnit::Absolute));
    background.set_destination_rect(RelativeRect::new(0.0, 0.0, 4.0, 5.0, RelativeUnit::Absolute));
    background.set_tile_mode(TileMode::Tile);
    background.set_stretch(Stretch::UniformToFill);
    let canvas = Canvas::new();
    canvas.set_width(4.0);
    canvas.set_height(5.0);
    canvas.set_background(Some(Brushes::white_smoke()));
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_left(&rectangle, 2.0);
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_top(&rectangle, 2.0);
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(1.0);
    rectangle.set_height(1.0);
    rectangle.set_fill(Some(Brushes::red()));
    Canvas::set_left(&rectangle, 2.0);
    Canvas::set_top(&rectangle, 4.0);
    canvas.children().add(rectangle);
    background.set_visual(canvas);
    target.set_background(Some(background.into()));

    t.render_to_file_with_dpi(&target, "VisualBrush_Grip_192_Dpi", 192.0);
    t.compare_images("VisualBrush_Grip_192_Dpi");
}

#[test]
fn visual_brush_checkerboard_96_dpi() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let background = VisualBrush::new();
    background.set_destination_rect(RelativeRect::new(0.0, 0.0, 16.0, 16.0, RelativeUnit::Absolute));
    background.set_tile_mode(TileMode::Tile);
    let canvas = Canvas::new();
    canvas.set_width(16.0);
    canvas.set_height(16.0);
    canvas.set_background(Some(Brushes::red()));
    let rectangle = Rectangle::new();
    rectangle.set_width(8.0);
    rectangle.set_height(8.0);
    rectangle.set_fill(Some(Brushes::green()));
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(8.0);
    rectangle.set_height(8.0);
    rectangle.set_fill(Some(Brushes::green()));
    Canvas::set_left(&rectangle, 8.0);
    Canvas::set_top(&rectangle, 8.0);
    canvas.children().add(rectangle);
    background.set_visual(canvas);
    target.set_background(Some(background.into()));

    t.render_to_file(&target, "VisualBrush_Checkerboard_96_Dpi");
    t.compare_images("VisualBrush_Checkerboard_96_Dpi");
}

#[test]
fn visual_brush_checkerboard_144_dpi() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let background = VisualBrush::new();
    background.set_destination_rect(RelativeRect::new(0.0, 0.0, 16.0, 16.0, RelativeUnit::Absolute));
    background.set_tile_mode(TileMode::Tile);
    let canvas = Canvas::new();
    canvas.set_width(16.0);
    canvas.set_height(16.0);
    canvas.set_background(Some(Brushes::red()));
    let rectangle = Rectangle::new();
    rectangle.set_width(8.0);
    rectangle.set_height(8.0);
    rectangle.set_fill(Some(Brushes::green()));
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(8.0);
    rectangle.set_height(8.0);
    rectangle.set_fill(Some(Brushes::green()));
    Canvas::set_left(&rectangle, 8.0);
    Canvas::set_top(&rectangle, 8.0);
    canvas.children().add(rectangle);
    background.set_visual(canvas);
    target.set_background(Some(background.into()));

    t.render_to_file_with_dpi(&target, "VisualBrush_Checkerboard_144_Dpi", 144.0);
    t.compare_images("VisualBrush_Checkerboard_144_Dpi");
}

#[test]
fn visual_brush_checkerboard_192_dpi() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let background = VisualBrush::new();
    background.set_destination_rect(RelativeRect::new(0.0, 0.0, 16.0, 16.0, RelativeUnit::Absolute));
    background.set_tile_mode(TileMode::Tile);
    let canvas = Canvas::new();
    canvas.set_width(16.0);
    canvas.set_height(16.0);
    canvas.set_background(Some(Brushes::red()));
    let rectangle = Rectangle::new();
    rectangle.set_width(8.0);
    rectangle.set_height(8.0);
    rectangle.set_fill(Some(Brushes::green()));
    canvas.children().add(rectangle);
    let rectangle = Rectangle::new();
    rectangle.set_width(8.0);
    rectangle.set_height(8.0);
    rectangle.set_fill(Some(Brushes::green()));
    Canvas::set_left(&rectangle, 8.0);
    Canvas::set_top(&rectangle, 8.0);
    canvas.children().add(rectangle);
    background.set_visual(canvas);
    target.set_background(Some(background.into()));

    t.render_to_file_with_dpi(&target, "VisualBrush_Checkerboard_192_Dpi", 192.0);
    t.compare_images("VisualBrush_Checkerboard_192_Dpi");
}

fn visual_brush_is_properly_mapped(relative: bool) {
    let t = base();
    let brush = VisualBrush::new();
    brush.set_stretch(Stretch::Fill);
    brush.set_tile_mode(TileMode::Tile);
    brush.set_destination_rect(if relative {
        RelativeRect::new(0.0, 0.0, 1.0, 1.0, RelativeUnit::Relative)
    } else {
        RelativeRect::new(0.0, 0.0, 256.0, 256.0, RelativeUnit::Absolute)
    });
    brush.set_visual(visual(&t));

    let test_name = format!("VisualBrush_Is_Properly_Mapped_{:?}", brush.destination_rect().unit);
    t.render_to_file(RelativePointTestPrimitivesHelper::new(Some(brush.into())), &test_name);
    t.compare_images(&test_name);
}

#[test]
fn visual_brush_is_properly_mapped_false() {
    visual_brush_is_properly_mapped(false);
}

#[test]
fn visual_brush_is_properly_mapped_true() {
    visual_brush_is_properly_mapped(true);
}

#[test]
fn visual_brush_should_be_usable_as_opacity_mask() {
    let t = base();
    let target = Border::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(920.0);
    target.set_height(920.0);
    target.set_background(Some(Brushes::magenta()));
    let opacity_mask = VisualBrush::new();
    opacity_mask.set_stretch(Stretch::Fill);
    opacity_mask.set_tile_mode(TileMode::None);

    let visual = Border::new();
    visual.set_width(200.0);
    visual.set_height(200.0);
    visual.set_padding(Thickness::uniform(20.0));
    let grid = Grid::new();

    grid.set_column_definitions(ColumnDefinitions::parse("*,*,*").unwrap());
    grid.set_row_definitions(RowDefinitions::parse("*,*,*").unwrap());
    let border = Border::new();
    border.set_background(Some(Brushes::aqua()));
    grid.children().add(border);
    let border = Border::new();
    Grid::set_column(&border, 1);
    Grid::set_row(&border, 2);
    border.set_background(Some(Brushes::aqua()));
    grid.children().add(border);
    let border = Border::new();
    Grid::set_column(&border, 3);
    border.set_background(Some(Brushes::aqua()));
    grid.children().add(border);
    visual.set_child(grid);
    opacity_mask.set_visual(visual);
    target.set_opacity_mask(Some(opacity_mask.into()));

    t.render_to_file(&target, "VisualBrush_Should_Be_Usable_As_Opacity_Mask");
    t.compare_images("VisualBrush_Should_Be_Usable_As_Opacity_Mask");
}
