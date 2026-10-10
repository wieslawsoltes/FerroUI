//! Port of upstream's `Shapes/RectangleTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::Brushes;
use ferroui_base::Thickness;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::Decorator;

fn base() -> TestBase {
    TestBase::new(r"Shapes\Rectangle")
}

#[test]
fn rectangle_0px_stroke() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_fill(Some(Brushes::transparent()));
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(0.0);
    target.set_child(child);

    t.render_to_file(&target, "Rectangle_0px_Stroke");
    t.compare_images("Rectangle_0px_Stroke");
}

#[test]
fn rectangle_1px_stroke() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(1.0);
    target.set_child(child);

    t.render_to_file(&target, "Rectangle_1px_Stroke");
    t.compare_images("Rectangle_1px_Stroke");
}

#[test]
fn rectangle_2px_stroke() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(2.0);
    target.set_child(child);

    t.render_to_file(&target, "Rectangle_2px_Stroke");
    t.compare_images("Rectangle_2px_Stroke");
}

#[test]
fn rectangle_stroke_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(2.0);
    child.set_fill(Some(Brushes::red()));
    target.set_child(child);

    t.render_to_file(&target, "Rectangle_Stroke_Fill");
    t.compare_images("Rectangle_Stroke_Fill");
}

#[test]
fn rectangle_stroke_fill_clip_to_bounds() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Rectangle::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(2.0);
    child.set_fill(Some(Brushes::red()));
    child.set_clip_to_bounds(true);
    target.set_child(child);

    t.render_to_file(&target, "Rectangle_Stroke_Fill_ClipToBounds");
    t.compare_images("Rectangle_Stroke_Fill_ClipToBounds");
}
