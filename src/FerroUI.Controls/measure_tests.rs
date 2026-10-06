//! Port of `MeasureTests.cs` (base unit tests). The tests measure controls,
//! so they live with the controls.

use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, ControlImpl, Decorator, StackPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, Thickness, Visual,
    VisualImpl,
};
use std::cell::Cell;

#[test]
fn style_hiding_control_should_be_applied_before_measuring() {
    let _scope = test_scope();
    let child = Border::new();
    child.set_width(100.0);
    child.set_height(100.0);
    child.classes().add("hidden");
    let target = Decorator::new();
    target.set_child(&child);
    let root = TestRoot::with_child(&target);

    root.styles().add(Style::with_setters(
        Selectors::of_type::<Border>().class("hidden"),
        [Setter::new(Visual::is_visible_property(), false)],
    ));

    target.measure(Size::INFINITY);

    assert!(!child.is_visible());
    assert_eq!(Size::new(0.0, 0.0), child.desired_size());
    assert_eq!(Size::new(0.0, 0.0), target.desired_size());
}

#[test]
fn margin_should_be_included_in_desired_size() {
    let _scope = test_scope();
    let decorator = Decorator::new();
    decorator.set_width(100.0);
    decorator.set_height(100.0);
    decorator.set_margin(Thickness::uniform(8.0));

    decorator.measure(Size::INFINITY);

    assert_eq!(Size::new(116.0, 116.0), decorator.desired_size());
}

#[test]
fn invalidating_child_should_not_invalidate_parent() {
    let _scope = test_scope();
    let panel = StackPanel::new();
    let child = Border::new();
    panel.children().add(child.clone());

    panel.measure(Size::new(f64::INFINITY, f64::INFINITY));

    assert_eq!(Size::new(0.0, 0.0), panel.desired_size());

    child.set_width(100.0);
    child.set_height(100.0);

    assert!(panel.is_measure_valid());
    assert!(!child.is_measure_valid());

    panel.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(Size::new(0.0, 0.0), panel.desired_size());
}

#[test]
fn removing_from_parent_should_invalidate_measure_of_control_and_descendants() {
    let _scope = test_scope();
    let panel = StackPanel::new();
    let child2 = Border::new();
    let child1 = Border::new();
    child1.set_child(&child2);
    panel.children().add(child1.clone());

    panel.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert!(child1.is_measure_valid());
    assert!(child2.is_measure_valid());

    panel.children().remove(&child1);
    assert!(!child1.is_measure_valid());
    assert!(!child2.is_measure_valid());
}

#[test]
fn negative_margin_larger_than_constraint_should_request_width_0() {
    let _scope = test_scope();
    let target = Control::new();
    target.set_margin(Thickness::new(-100.0, 0.0, 0.0, 0.0));
    let outer = Decorator::new();
    outer.set_width(100.0);
    outer.set_height(100.0);
    outer.set_child(&target);

    outer.measure(Size::INFINITY);

    assert_eq!(0.0, target.desired_size().width);
}

#[test]
fn negative_margin_larger_than_constraint_should_request_height_0() {
    let _scope = test_scope();
    let target = Control::new();
    target.set_margin(Thickness::new(0.0, -100.0, 0.0, 0.0));
    let outer = Decorator::new();
    outer.set_width(100.0);
    outer.set_height(100.0);
    outer.set_child(&target);

    outer.measure(Size::INFINITY);

    assert_eq!(0.0, target.desired_size().height);
}

#[test]
fn margin_should_affect_available_size() {
    let _scope = test_scope();
    let target = MeasureTest::new();
    target.set_margin(Thickness::uniform(10.0));
    let outer = Decorator::new();
    outer.set_width(100.0);
    outer.set_height(100.0);
    outer.set_child(&target);

    outer.measure(Size::INFINITY);

    assert_eq!(Some(Size::new(80.0, 80.0)), target.available_size.get());
}

#[test]
fn margin_should_be_applied_before_width_height() {
    let _scope = test_scope();
    let target = MeasureTest::new();
    target.set_width(80.0);
    target.set_height(80.0);
    target.set_margin(Thickness::uniform(10.0));
    let outer = Decorator::new();
    outer.set_width(100.0);
    outer.set_height(100.0);
    outer.set_child(&target);

    outer.measure(Size::INFINITY);

    assert_eq!(Some(Size::new(80.0, 80.0)), target.available_size.get());
}

#[repr(C)]
struct MeasureTest {
    base: Control,
    available_size: Cell<Option<Size>>,
}

ferro_class!(MeasureTest: Control);
ferro_impl_classes!(
    MeasureTest: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for MeasureTest {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.available_size.set(Some(available_size));
        available_size
    }
}

impl MeasureTest {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), available_size: Cell::new(None) })
    }
}
