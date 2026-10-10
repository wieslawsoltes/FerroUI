//! Port of upstream's `OpacityMaskTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, Color, GradientStop, LinearGradientBrush, RotateTransform, Stretch, StreamGeometry};
use ferroui_base::{RelativePoint, RelativeUnit};
use ferroui_controls::shapes::Path;
use ferroui_controls::Canvas;

fn base() -> TestBase {
    TestBase::new("OpacityMask")
}

#[test]
fn opacity_mask_masks_element() {
    let t = base();
    let target = Canvas::new();
    let opacity_mask = LinearGradientBrush::new();
    opacity_mask.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
    opacity_mask.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
    opacity_mask.gradient_stops().add(GradientStop::with_color_and_offset(Color::from_uint32(0xffffffff), 0.0));
    opacity_mask.gradient_stops().add(GradientStop::with_color_and_offset(Color::from_uint32(0x00ffffff), 1.0));
    target.set_opacity_mask(Some(opacity_mask.into()));
    target.set_width(76.0);
    target.set_height(76.0);
    target.set_background(Some(Brushes::transparent()));
    let path = Path::new();
    path.set_width(32.0);
    path.set_height(40.0);
    Canvas::set_left(&path, 23.0);
    Canvas::set_top(&path, 18.0);
    path.set_stretch(Stretch::Fill);
    path.set_fill(Some(Brushes::red()));
    path.set_data(
        StreamGeometry::parse("F1 M 27,18L 23,26L 33,30L 24,38L 33,46L 23,50L 27,58L 45,58L 55,38L 45,18L 27,18 Z")
            .expect("the path data is valid"),
    );
    target.children().add(path);

    t.render_to_file(&target, "Opacity_Mask_Masks_Element");
    t.compare_images("Opacity_Mask_Masks_Element");
}

#[test]
fn render_transform_applies_to_opacity_mask() {
    let t = base();
    let target = Canvas::new();
    let opacity_mask = LinearGradientBrush::new();
    opacity_mask.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
    opacity_mask.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
    opacity_mask.gradient_stops().add(GradientStop::with_color_and_offset(Color::from_uint32(0xffffffff), 0.0));
    opacity_mask.gradient_stops().add(GradientStop::with_color_and_offset(Color::from_uint32(0x00ffffff), 1.0));
    target.set_opacity_mask(Some(opacity_mask.into()));
    target.set_render_transform(Some(RotateTransform::with_angle(90.0).into()));
    target.set_width(76.0);
    target.set_height(76.0);
    target.set_background(Some(Brushes::transparent()));
    let path = Path::new();
    path.set_width(32.0);
    path.set_height(40.0);
    Canvas::set_left(&path, 23.0);
    Canvas::set_top(&path, 18.0);
    path.set_stretch(Stretch::Fill);
    path.set_fill(Some(Brushes::red()));
    path.set_data(
        StreamGeometry::parse("F1 M 27,18L 23,26L 33,30L 24,38L 33,46L 23,50L 27,58L 45,58L 55,38L 45,18L 27,18 Z")
            .expect("the path data is valid"),
    );
    target.children().add(path);

    t.render_to_file(&target, "RenderTransform_Applies_To_Opacity_Mask");
    t.compare_images("RenderTransform_Applies_To_Opacity_Mask");
}
