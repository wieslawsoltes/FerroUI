//! Port of `ArrangeTests.cs` (base unit tests). The tests arrange
//! decorators, so they live with the controls.

use crate::{ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, LayoutableImplExt, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, Thickness,
    VisualImpl,
};
use std::cell::Cell;

#[test]
fn bounds_should_not_include_margin() {
    let target = Decorator::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_margin(Thickness::uniform(5.0));

    assert!(!target.is_measure_valid());
    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert_eq!(Rect::new(5.0, 5.0, 100.0, 100.0), target.bounds());
}

#[test]
fn margin_should_be_subtracted_from_arrange_final_size() {
    let target = TestControl::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_margin(Thickness::uniform(8.0));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Size::new(100.0, 100.0), target.arrange_final_size.get());
}

#[test]
fn arrange_override_receives_desired_size_when_centered() {
    let target = TestControl::new();
    target.measure_result.set(Size::new(100.0, 100.0));
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_margin(Thickness::uniform(8.0));

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    assert_eq!(Size::new(100.0, 100.0), target.arrange_final_size.get());
}

#[test]
fn arrange_override_receives_available_size_minus_margin_when_stretched() {
    let target = TestControl::new();
    target.measure_result.set(Size::new(100.0, 100.0));
    target.set_horizontal_alignment(HorizontalAlignment::Stretch);
    target.set_vertical_alignment(VerticalAlignment::Stretch);
    target.set_margin(Thickness::uniform(8.0));

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    assert_eq!(Size::new(184.0, 184.0), target.arrange_final_size.get());
}

#[test]
fn arrange_override_receives_requested_size_when_arranged_to_desired_size() {
    let target = TestControl::new();
    target.measure_result.set(Size::new(100.0, 100.0));
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_margin(Thickness::uniform(8.0));

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Size::new(100.0, 100.0), target.arrange_final_size.get());
}

#[test]
fn arrange_with_is_measure_valid_false_calls_measure() {
    let target = TestControl::new();

    assert!(!target.is_measure_valid());
    target.arrange(Rect::new(0.0, 0.0, 120.0, 120.0));
    assert!(target.is_measure_valid());
    assert_eq!(Size::new(120.0, 120.0), target.measure_constraint.get());
}

#[test]
fn arrange_with_is_measure_valid_false_calls_measure_with_previous_size_if_available() {
    let target = TestControl::new();

    assert!(!target.is_measure_valid());
    target.arrange(Rect::new(0.0, 0.0, 120.0, 120.0));
    target.invalidate_measure();
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    assert!(target.is_measure_valid());
    assert_eq!(Size::new(120.0, 120.0), target.measure_constraint.get());
}

#[repr(C)]
struct TestControl {
    base: Decorator,
    measure_constraint: Cell<Size>,
    measure_result: Cell<Size>,
    arrange_final_size: Cell<Size>,
}

ferro_class!(TestControl: Decorator);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for TestControl {
    fn measure_override(this: &Self, constraint: Size) -> Size {
        this.measure_constraint.set(constraint);
        this.measure_result.get()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arrange_final_size.set(final_size);
        Self::parent_arrange_override(this, final_size)
    }
}

impl TestControl {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Decorator::construct(),
            measure_constraint: Cell::new(Size::default()),
            measure_result: Cell::new(Size::default()),
            arrange_final_size: Cell::new(Size::default()),
        })
    }
}
