//! Port of upstream's `Shapes/LineTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, MediaCollection};
use ferroui_base::{Point, Ref, Thickness};
use ferroui_controls::shapes::Line;
use ferroui_controls::{Decorator, StackPanel};

fn base() -> TestBase {
    TestBase::new(r"Shapes\Line")
}

#[test]
fn line_1px_stroke() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Line::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(1.0);
    child.set_start_point(Point::new(0.0, 0.0));
    child.set_end_point(Point::new(200.0, 200.0));
    target.set_child(child);

    t.render_to_file(&target, "Line_1px_Stroke");
    t.compare_images("Line_1px_Stroke");
}

#[test]
fn line_1px_stroke_reversed() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Line::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(1.0);
    child.set_start_point(Point::new(200.0, 0.0));
    child.set_end_point(Point::new(0.0, 200.0));
    target.set_child(child);

    t.render_to_file(&target, "Line_1px_Stroke_Reversed");
    t.compare_images("Line_1px_Stroke_Reversed");
}

#[test]
fn line_1px_stroke_vertical() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Line::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(1.0);
    child.set_start_point(Point::new(100.0, 200.0));
    child.set_end_point(Point::new(100.0, 0.0));
    target.set_child(child);

    t.render_to_file(&target, "Line_1px_Stroke_Vertical");
    t.compare_images("Line_1px_Stroke_Vertical");
}

/// One line of `Lines_With_DashArray`: upstream writes the same object
/// initializer ten times, with a different dash array.
fn dashed_line<const N: usize>(stroke_dash_array: [f64; N]) -> Ref<Line> {
    let line = Line::new();
    line.set_margin(Thickness::uniform(8.0));
    line.set_stroke_thickness(8.0);
    line.set_start_point(Point::new(0.0, 0.0));
    line.set_end_point(Point::new(200.0, 0.0));
    line.set_stroke(Some(Brushes::black()));
    line.set_stroke_dash_array(Some(MediaCollection::from_items(stroke_dash_array)));
    line
}

#[test]
fn lines_with_dash_array() {
    let t = base();
    let stack_panel = StackPanel::new();

    stack_panel.children().add(dashed_line([1.0]));
    stack_panel.children().add(dashed_line([1.0, 1.0]));
    stack_panel.children().add(dashed_line([1.0, 6.0]));
    stack_panel.children().add(dashed_line([6.0, 1.0]));
    stack_panel.children().add(dashed_line([0.25, 1.0]));
    stack_panel.children().add(dashed_line([4.0, 1.0, 1.0, 1.0, 1.0, 1.0]));
    stack_panel.children().add(dashed_line([5.0, 5.0, 1.0, 5.0]));
    stack_panel.children().add(dashed_line([1.0, 2.0, 4.0]));
    stack_panel.children().add(dashed_line([4.0, 2.0, 4.0]));
    stack_panel.children().add(dashed_line([4.0, 2.0, 4.0, 1.0, 1.0]));

    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_child(stack_panel);

    t.render_to_file(&target, "Lines_With_DashArray");
    t.compare_images("Lines_With_DashArray");
}
