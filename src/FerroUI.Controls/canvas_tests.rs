// The reference tests use a rectangle shape as the positioned child; any
// control with a minimum size shows the same behaviour, so a plain control is
// used here.
use crate::{Canvas, Control};
use ferroui_base::{Rect, Ref, Size};

fn canvas_with_child() -> (Ref<Canvas>, Ref<Control>) {
    let rect = Control::new();
    rect.set_min_width(20.0);
    rect.set_min_height(25.0);

    let target = Canvas::new();
    target.set_width(400.0);
    target.set_height(400.0);
    target.children().add(&rect);

    (target, rect)
}

#[test]
fn left_property_should_work() {
    let (target, rect) = canvas_with_child();
    Canvas::set_left(&rect, 30.0);

    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(rect.bounds(), Rect::new(30.0, 0.0, 20.0, 25.0));
}

#[test]
fn top_property_should_work() {
    let (target, rect) = canvas_with_child();
    Canvas::set_top(&rect, 30.0);

    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(rect.bounds(), Rect::new(0.0, 30.0, 20.0, 25.0));
}

#[test]
fn right_property_should_work() {
    let (target, rect) = canvas_with_child();
    Canvas::set_right(&rect, 30.0);

    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(rect.bounds(), Rect::new(350.0, 0.0, 20.0, 25.0));
}

#[test]
fn bottom_property_should_work() {
    let (target, rect) = canvas_with_child();
    Canvas::set_bottom(&rect, 30.0);

    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(rect.bounds(), Rect::new(0.0, 345.0, 20.0, 25.0));
}
