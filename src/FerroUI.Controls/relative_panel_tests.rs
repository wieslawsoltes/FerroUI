// The reference tests use rectangle shapes as the children; any control with
// an explicit size shows the same behaviour, so borders are used here.
use crate::{Border, Control, RelativePanel};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{Rect, Ref, Size};

fn rect() -> Ref<Border> {
    let rect = Border::new();
    rect.set_height(20.0);
    rect.set_width(20.0);
    rect
}

fn panel(
    vertical_alignment: VerticalAlignment,
    horizontal_alignment: HorizontalAlignment,
) -> (Ref<RelativePanel>, Ref<Border>, Ref<Border>) {
    let rect1 = rect();
    let rect2 = rect();

    let target = RelativePanel::new();
    target.set_vertical_alignment(vertical_alignment);
    target.set_horizontal_alignment(horizontal_alignment);
    target.children().add(&rect1);
    target.children().add(&rect2);

    (target, rect1, rect2)
}

#[test]
fn lays_out_1_child_next_the_other() {
    let (target, rect1, rect2) = panel(VerticalAlignment::Top, HorizontalAlignment::Left);

    RelativePanel::set_align_left_with_panel(&rect1, true);
    RelativePanel::set_right_of(&rect2, Some(Control::boxed(&rect1)));
    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(40.0, 20.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(20.0, 0.0, 20.0, 20.0));
}

#[test]
fn lays_out_1_child_below_the_other() {
    let (target, rect1, rect2) = panel(VerticalAlignment::Top, HorizontalAlignment::Left);

    RelativePanel::set_align_left_with_panel(&rect1, true);
    RelativePanel::set_below(&rect2, Some(Control::boxed(&rect1)));
    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(20.0, 40.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 20.0, 20.0, 20.0));
}

#[test]
fn relative_panel_can_center() {
    let (target, rect1, rect2) = panel(VerticalAlignment::Center, HorizontalAlignment::Center);

    RelativePanel::set_align_left_with_panel(&rect1, true);
    RelativePanel::set_below(&rect2, Some(Control::boxed(&rect1)));
    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(20.0, 40.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 20.0, 20.0, 20.0));
}

#[test]
fn left_of_measures_correctly() {
    let (target, rect1, rect2) = panel(VerticalAlignment::Center, HorizontalAlignment::Center);

    RelativePanel::set_left_of(&rect2, Some(Control::boxed(&rect1)));
    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(20.0, 20.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(-20.0, 0.0, 20.0, 20.0));
}

#[test]
fn above_measures_correctly() {
    let (target, rect1, rect2) = panel(VerticalAlignment::Center, HorizontalAlignment::Center);

    RelativePanel::set_above(&rect2, Some(Control::boxed(&rect1)));
    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds().size(), Size::new(20.0, 20.0));
    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, -20.0, 20.0, 20.0));
}

#[test]
fn stretched_panel_measures_correctly() {
    for (available_width, available_height, desired_width, desired_height) in [
        (100.0, 100.0, 100.0, 100.0),
        (100.0, f64::INFINITY, 100.0, 40.0),
        (f64::INFINITY, 100.0, 20.0, 100.0),
        (f64::INFINITY, f64::INFINITY, 20.0, 40.0),
    ] {
        let (target, rect1, rect2) = panel(VerticalAlignment::Stretch, HorizontalAlignment::Stretch);

        RelativePanel::set_below(&rect2, Some(Control::boxed(&rect1)));
        target.measure(Size::new(available_width, available_height));
        target.arrange(Rect::from_size(target.desired_size()));

        assert_eq!(target.desired_size().width, desired_width);
        assert_eq!(target.desired_size().height, desired_height);
    }
}

#[test]
#[should_panic(expected = "Element does not exist in the current context: RightOf")]
fn dependency_outside_the_panel_panics() {
    let (target, _rect1, rect2) = panel(VerticalAlignment::Top, HorizontalAlignment::Left);

    RelativePanel::set_right_of(&rect2, Some(Control::boxed(rect())));
    target.measure(Size::new(400.0, 400.0));
}

#[test]
#[should_panic(expected = "Circular dependency detected")]
fn circular_dependency_panics() {
    let (target, rect1, rect2) = panel(VerticalAlignment::Top, HorizontalAlignment::Left);

    RelativePanel::set_right_of(&rect2, Some(Control::boxed(&rect1)));
    RelativePanel::set_right_of(&rect1, Some(Control::boxed(&rect2)));
    target.measure(Size::new(400.0, 400.0));
}
