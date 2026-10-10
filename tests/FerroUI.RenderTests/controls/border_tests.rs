//! Port of upstream's `Controls/BorderTests.cs`.
//!
//! The tests upstream marks `Win32Fact("Has text")` run only on Windows
//! there, and here.

use crate::test_base::TestBase;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, FontFamily, RotateTransform};
use ferroui_base::{CornerRadius, Thickness};
use ferroui_controls::{Border, Decorator, TextBlock};

fn base() -> TestBase {
    TestBase::new(r"Controls\Border")
}

#[test]
fn border_1px_border() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(1.0));
    target.set_child(child);

    t.render_to_file(&target, "Border_1px_Border");
    t.compare_images("Border_1px_Border");
}

#[test]
fn border_2px_border() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    target.set_child(child);

    t.render_to_file(&target, "Border_2px_Border");
    t.compare_images("Border_2px_Border");
}

#[test]
fn border_uniform_corner_radius() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    child.set_corner_radius(CornerRadius::uniform(16.0));
    target.set_child(child);

    t.render_to_file(&target, "Border_Uniform_CornerRadius");
    t.compare_images("Border_Uniform_CornerRadius");
}

#[test]
fn border_non_uniform_corner_radius() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    child.set_corner_radius(CornerRadius::new(16.0, 4.0, 7.0, 10.0));
    target.set_child(child);

    t.render_to_file(&target, "Border_NonUniform_CornerRadius");
    t.compare_images("Border_NonUniform_CornerRadius");
}

#[test]
fn border_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_background(Some(Brushes::red()));
    target.set_child(child);

    t.render_to_file(&target, "Border_Fill");
    t.compare_images("Border_Fill");
}

#[test]
fn border_brush_offsets_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = Border::new();
    content.set_background(Some(Brushes::red()));
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Brush_Offsets_Content");
    t.compare_images("Border_Brush_Offsets_Content");
}

#[test]
fn border_padding_offsets_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    child.set_padding(Thickness::uniform(2.0));
    let content = Border::new();
    content.set_background(Some(Brushes::red()));
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Padding_Offsets_Content");
    t.compare_images("Border_Padding_Offsets_Content");
}

#[test]
fn border_margin_offsets_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = Border::new();
    content.set_background(Some(Brushes::red()));
    content.set_margin(Thickness::uniform(2.0));
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Margin_Offsets_Content");
    t.compare_images("Border_Margin_Offsets_Content");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn border_centers_content_horizontally() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Centers_Content_Horizontally");
    t.compare_images("Border_Centers_Content_Horizontally");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn border_centers_content_vertically() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_vertical_alignment(VerticalAlignment::Center);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Centers_Content_Vertically");
    t.compare_images("Border_Centers_Content_Vertically");
}

#[test]
fn border_stretches_content_horizontally() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_horizontal_alignment(HorizontalAlignment::Stretch);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Stretches_Content_Horizontally");
    t.compare_images("Border_Stretches_Content_Horizontally");
}

#[test]
fn border_stretches_content_vertically() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_vertical_alignment(VerticalAlignment::Stretch);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Stretches_Content_Vertically");
    t.compare_images("Border_Stretches_Content_Vertically");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn border_left_aligns_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_horizontal_alignment(HorizontalAlignment::Left);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Left_Aligns_Content");
    t.compare_images("Border_Left_Aligns_Content");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn border_right_aligns_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_horizontal_alignment(HorizontalAlignment::Right);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Right_Aligns_Content");
    t.compare_images("Border_Right_Aligns_Content");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn border_top_aligns_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_vertical_alignment(VerticalAlignment::Top);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Top_Aligns_Content");
    t.compare_images("Border_Top_Aligns_Content");
}

#[test]
#[cfg_attr(not(windows), ignore = "Has text")]
fn border_bottom_aligns_content() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_border_brush(Some(Brushes::black()));
    child.set_border_thickness(Thickness::uniform(2.0));
    let content = TextBlock::new();
    content.set_text(Some("Foo"));
    content.set_background(Some(Brushes::red()));
    content.set_font_family(FontFamily::new("Segoe UI"));
    content.set_font_size(12.0);
    content.set_vertical_alignment(VerticalAlignment::Bottom);
    child.set_child(content);
    target.set_child(child);

    t.render_to_file(&target, "Border_Bottom_Aligns_Content");
    t.compare_images("Border_Bottom_Aligns_Content");
}

#[test]
fn border_nested_rotate() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_background(Some(Brushes::coral()));
    child.set_width(100.0);
    child.set_height(100.0);
    child.set_horizontal_alignment(HorizontalAlignment::Center);
    child.set_vertical_alignment(VerticalAlignment::Center);
    let content = Border::new();
    content.set_margin(Thickness::uniform(25.0));
    content.set_background(Some(Brushes::chocolate()));
    child.set_child(content);
    child.set_render_transform(Some(RotateTransform::with_angle(45.0).into()));
    target.set_child(child);

    t.render_to_file(&target, "Border_Nested_Rotate");
    t.compare_images("Border_Nested_Rotate");
}

fn border_clips_to_round_bounds(uniform: bool) {
    let t = base();
    let corner_radius = if uniform { CornerRadius::uniform(20.0) } else { CornerRadius::new(20.0, 10.0, 30.0, 5.0) };

    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Border::new();
    child.set_corner_radius(corner_radius);
    child.set_background(Some(Brushes::light_blue()));
    child.set_clip_to_bounds(true);
    let content = Border::new();
    content.set_width(300.0);
    content.set_height(300.0);
    content.set_background(Some(Brushes::red()));
    child.set_child(content);
    target.set_child(child);

    let test_suffix = format!("Border_Clips_To_Round_Bounds_{}", if uniform { "Uniform" } else { "NonUniform" });
    t.render_to_file(&target, &test_suffix);
    t.compare_images(&test_suffix);
}

#[test]
fn border_clips_to_round_bounds_true() {
    border_clips_to_round_bounds(true);
}

#[test]
fn border_clips_to_round_bounds_false() {
    border_clips_to_round_bounds(false);
}
