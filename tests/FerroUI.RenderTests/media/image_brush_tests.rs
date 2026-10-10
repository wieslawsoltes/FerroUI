//! Port of upstream's `Media/ImageBrushTests.cs`.
//!
//! Upstream loads the source images from its output directory, which is
//! also the directory of its expected images; the port loads them from the
//! directory of the expected images.

use crate::media::relative_point_test_primitives_helper::RelativePointTestPrimitivesHelper;
use crate::test_base::TestBase;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{
    AlignmentX, AlignmentY, Brushes, DrawingBrush, DrawingImage, GeometryDrawing, ImageBrush, ImageDrawing,
    RectangleGeometry, RotateTransform, Stretch, TileMode, TranslateTransform,
};
use ferroui_base::{Rect, RelativePoint, RelativeRect, RelativeUnit, Thickness};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Decorator, Image};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\ImageBrush")
}

fn bitmap_path(t: &TestBase) -> String {
    t.expected_path().join("github_icon.png").to_str().expect("the path is text").to_string()
}

fn small_bitmap_path(t: &TestBase) -> String {
    t.expected_path().join("github_icon_small.png").to_str().expect("the path is text").to_string()
}

/// `new Bitmap(path)`.
fn bitmap(path: &str) -> Rc<Bitmap> {
    Rc::new(Bitmap::from_file(path).expect("the image is loaded"))
}

#[test]
fn image_brush_null_source() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_margin(Thickness::uniform(8.0));
    child.set_fill(Some(ImageBrush::new().into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NullSource");
    t.compare_images("ImageBrush_NullSource");
}

#[test]
fn image_brush_tile_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_margin(Thickness::uniform(8.0));
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::Fill);
    fill.set_tile_mode(TileMode::Tile);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 25.0, 30.0, RelativeUnit::Absolute));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_Tile_Fill");
    t.compare_images("ImageBrush_Tile_Fill");
}

#[test]
fn image_brush_tile_uniform_to_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_margin(Thickness::uniform(8.0));
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::Uniform);
    fill.set_tile_mode(TileMode::Tile);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 25.0, 30.0, RelativeUnit::Absolute));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_Tile_UniformToFill");
    t.compare_images("ImageBrush_Tile_UniformToFill");
}

#[test]
fn image_brush_tile_small_image() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_margin(Thickness::uniform(8.0));
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::Tile);
    fill.set_source(Some(bitmap(&small_bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_Tile_Small_Image");
    t.compare_images("ImageBrush_Tile_Small_Image");
}

#[test]
fn image_brush_no_stretch_no_tile_alignment_top_left() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_alignment_x(AlignmentX::Left);
    fill.set_alignment_y(AlignmentY::Top);
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_NoTile_Alignment_TopLeft");
    t.compare_images("ImageBrush_NoStretch_NoTile_Alignment_TopLeft");
}

#[test]
fn image_brush_no_stretch_no_tile_alignment_center() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_alignment_x(AlignmentX::Center);
    fill.set_alignment_y(AlignmentY::Center);
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_NoTile_Alignment_Center");
    t.compare_images("ImageBrush_NoStretch_NoTile_Alignment_Center");
}

#[test]
fn image_brush_no_stretch_no_tile_alignment_bottom_right() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_alignment_x(AlignmentX::Right);
    fill.set_alignment_y(AlignmentY::Bottom);
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_NoTile_Alignment_BottomRight");
    t.compare_images("ImageBrush_NoStretch_NoTile_Alignment_BottomRight");
}

#[test]
fn image_brush_fill_no_tile() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(920.0);
    target.set_height(920.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::Fill);
    fill.set_tile_mode(TileMode::None);
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_Fill_NoTile");
    t.compare_images("ImageBrush_Fill_NoTile");
}

#[test]
fn image_brush_uniform_no_tile() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(300.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::Uniform);
    fill.set_tile_mode(TileMode::None);
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_Uniform_NoTile");
    t.compare_images("ImageBrush_Uniform_NoTile");
}

#[test]
fn image_brush_uniform_to_fill_no_tile() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(300.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::UniformToFill);
    fill.set_tile_mode(TileMode::None);
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_UniformToFill_NoTile");
    t.compare_images("ImageBrush_UniformToFill_NoTile");
}

#[test]
fn image_brush_no_stretch_no_tile_bottom_right_quarter_source() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_source_rect(RelativeRect::new(250.0, 250.0, 250.0, 250.0, RelativeUnit::Absolute));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_NoTile_BottomRightQuarterSource");
    t.compare_images("ImageBrush_NoStretch_NoTile_BottomRightQuarterSource");
}

#[test]
fn image_brush_no_stretch_no_tile_bottom_right_quarter_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_destination_rect(RelativeRect::new(92.0, 92.0, 92.0, 92.0, RelativeUnit::Absolute));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_NoTile_BottomRightQuarterDest");
    t.compare_images("ImageBrush_NoStretch_NoTile_BottomRightQuarterDest");
}

#[test]
fn image_brush_no_stretch_no_tile_bottom_right_quarter_source_bottom_right_quarter_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::None);
    fill.set_source_rect(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_destination_rect(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_NoTile_BottomRightQuarterSource_BottomRightQuarterDest");
    t.compare_images("ImageBrush_NoStretch_NoTile_BottomRightQuarterSource_BottomRightQuarterDest");
}

#[test]
fn image_brush_no_stretch_tile_bottom_right_quarter_source_center_quarter_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::Tile);
    fill.set_source_rect(RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_destination_rect(RelativeRect::new(0.25, 0.25, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_Tile_BottomRightQuarterSource_CenterQuarterDest");
    t.compare_images("ImageBrush_NoStretch_Tile_BottomRightQuarterSource_CenterQuarterDest");
}

#[test]
fn image_brush_no_stretch_flip_x_top_left_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::FlipX);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_FlipX_TopLeftDest");
    t.compare_images("ImageBrush_NoStretch_FlipX_TopLeftDest");
}

#[test]
fn image_brush_no_stretch_flip_y_top_left_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::FlipY);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_FlipY_TopLeftDest");
    t.compare_images("ImageBrush_NoStretch_FlipY_TopLeftDest");
}

#[test]
fn image_brush_no_stretch_flip_xy_top_left_dest() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    let fill = ImageBrush::new();
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::FlipXY);
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    fill.set_source(Some(bitmap(&bitmap_path(&t))));
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_NoStretch_FlipXY_TopLeftDest");
    t.compare_images("ImageBrush_NoStretch_FlipXY_TopLeftDest");
}

fn image_brush_is_properly_mapped(relative: bool) {
    let t = base();
    let brush = ImageBrush::new();
    brush.set_stretch(Stretch::Fill);
    brush.set_tile_mode(TileMode::Tile);
    brush.set_destination_rect(if relative {
        RelativeRect::new(0.0, 0.0, 1.0, 1.0, RelativeUnit::Relative)
    } else {
        RelativeRect::new(0.0, 0.0, 256.0, 256.0, RelativeUnit::Absolute)
    });
    brush.set_source(Some(bitmap(&bitmap_path(&t))));

    let test_name = format!("ImageBrush_Is_Properly_Mapped_{:?}", brush.destination_rect().unit);
    t.render_to_file(RelativePointTestPrimitivesHelper::new(Some(brush.into())), &test_name);
    t.compare_images(&test_name);
}

#[test]
fn image_brush_is_properly_mapped_false() {
    image_brush_is_properly_mapped(false);
}

#[test]
fn image_brush_is_properly_mapped_true() {
    image_brush_is_properly_mapped(true);
}

#[test]
fn image_brush_should_render_with_transform() {
    let t = base();
    let image = Image::new();
    image.set_width(200.0);
    image.set_height(200.0);
    let source = DrawingImage::new();
    let drawing = GeometryDrawing::new();
    let brush = DrawingBrush::new();
    let transform = TranslateTransform::new();
    transform.set_x(10.0);
    transform.set_y(10.0);
    brush.set_transform(Some(transform.into()));
    let brush_drawing = GeometryDrawing::new();
    brush_drawing.set_brush(Some(Brushes::medium_blue()));
    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 48.0, 48.0));
    brush_drawing.set_geometry(geometry);
    brush.set_drawing(brush_drawing);
    drawing.set_brush(Some(brush.into()));
    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 48.0, 48.0));
    drawing.set_geometry(geometry);
    source.set_drawing(drawing);
    image.set_source(Some(source.into()));

    t.render_to_file(&image, "ImageBrush_Should_Render_With_Transform");

    t.compare_images("ImageBrush_Should_Render_With_Transform");
}

#[test]
fn image_brush_should_render_with_transform_origin() {
    let t = base();
    let image = Image::new();
    image.set_width(200.0);
    image.set_height(200.0);
    let source = DrawingImage::new();
    let drawing = GeometryDrawing::new();
    let brush = DrawingBrush::new();
    brush.set_transform(Some(RotateTransform::with_angle(45.0).into()));
    brush.set_transform_origin(RelativePoint::new(0.5, 0.5, RelativeUnit::Relative));
    let brush_drawing = GeometryDrawing::new();
    brush_drawing.set_brush(Some(Brushes::medium_blue()));
    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 48.0, 48.0));
    brush_drawing.set_geometry(geometry);
    brush.set_drawing(brush_drawing);
    drawing.set_brush(Some(brush.into()));
    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 48.0, 48.0));
    drawing.set_geometry(geometry);
    source.set_drawing(drawing);
    image.set_source(Some(source.into()));

    t.render_to_file(&image, "ImageBrush_Should_Render_With_TransformOrigin");

    t.compare_images("ImageBrush_Should_Render_With_TransformOrigin");
}

#[test]
fn image_brush_tile_small_image_with_transform() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_margin(Thickness::uniform(8.0));
    let fill = DrawingBrush::new();
    fill.set_destination_rect(RelativeRect::new(0.0, 0.0, 32.0, 32.0, RelativeUnit::Absolute));
    fill.set_transform(Some(TranslateTransform::with_offset(10.0, 10.0).into()));
    fill.set_stretch(Stretch::None);
    fill.set_tile_mode(TileMode::Tile);
    let drawing = ImageDrawing::new();
    drawing.set_rect(Rect::new(0.0, 0.0, 32.0, 32.0));
    drawing.set_image_source(Some(bitmap(&small_bitmap_path(&t))));
    fill.set_drawing(drawing);
    child.set_fill(Some(fill.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageBrush_Tile_Small_Image_With_Transform");
    t.compare_images("ImageBrush_Tile_Small_Image_With_Transform");
}
