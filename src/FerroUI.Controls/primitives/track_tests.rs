use crate::primitives::{Thumb, Track};
use crate::test_support::test_scope;
use crate::Button;
use ferroui_base::layout::Orientation;
use ferroui_base::{Rect, Ref, Size, StyledElement};

#[test]
fn measure_should_return_thumb_desired_width_in_vertical_orientation() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_width(12.0);

    let target = Track::new();
    target.set_thumb(thumb);
    target.set_orientation(Orientation::Vertical);

    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::new(12.0, 0.0), target.desired_size());
}

#[test]
fn measure_should_return_thumb_desired_height_in_horizontal_orientation() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_height(12.0);

    let target = Track::new();
    target.set_thumb(thumb);
    target.set_orientation(Orientation::Horizontal);

    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::new(0.0, 12.0), target.desired_size());
}

#[test]
fn should_arrange_thumb_in_horizontal_orientation() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_height(12.0);

    let target = Track::new();
    target.set_thumb(thumb.clone());
    target.set_orientation(Orientation::Horizontal);
    target.set_minimum(100.0);
    target.set_maximum(200.0);
    target.set_height(12.0);
    target.set_range_value(150.0);
    target.set_viewport_size(50.0);

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(33.0, 0.0, 34.0, 12.0), thumb.bounds());
}

#[test]
fn should_arrange_thumb_in_vertical_orientation() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_width(12.0);

    let target = Track::new();
    target.set_thumb(thumb.clone());
    target.set_orientation(Orientation::Vertical);
    target.set_minimum(100.0);
    target.set_maximum(200.0);
    target.set_range_value(150.0);
    target.set_viewport_size(50.0);
    target.set_width(12.0);

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(0.0, 33.0, 12.0, 34.0), thumb.bounds());
}

#[test]
fn thumb_should_have_zero_width_when_minimum_equals_maximum() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_height(12.0);

    let target = Track::new();
    target.set_height(12.0);
    target.set_thumb(thumb.clone());
    target.set_orientation(Orientation::Horizontal);
    target.set_minimum(100.0);
    target.set_maximum(100.0);

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(0.0, 0.0, 0.0, 12.0), thumb.bounds());
}

#[test]
fn thumb_should_be_logical_child() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_height(12.0);

    let target = Track::new();
    target.set_height(12.0);
    target.set_thumb(thumb.clone());
    target.set_orientation(Orientation::Horizontal);
    target.set_minimum(100.0);
    target.set_maximum(100.0);

    assert_eq!(thumb.parent().unwrap(), target);
    let expected: Vec<Ref<StyledElement>> = vec![thumb.clone().upcast()];
    assert_eq!(StyledElement::logical_children(&target).to_vec(), expected);
}

#[test]
fn should_not_pass_invalid_arrange_rect() {
    let _scope = test_scope();
    let thumb = Thumb::new();
    thumb.set_width(100.873106060606);
    let increase_button = Button::new();
    increase_button.set_width(10.0);
    let decrease_button = Button::new();
    decrease_button.set_width(10.0);

    let target = Track::new();
    target.set_height(12.0);
    target.set_thumb(thumb);
    target.set_increase_button(increase_button);
    target.set_decrease_button(decrease_button);
    target.set_orientation(Orientation::Horizontal);
    target.set_minimum(0.0);
    target.set_maximum(287.0);
    target.set_range_value(287.0);
    target.set_viewport_size(241.0);

    target.measure(Size::INFINITY);

    // An invalid rectangle was passed to arrange at this point in the
    // reference implementation.
    target.arrange(Rect::new(0.0, 0.0, 221.0, 12.0));
}
