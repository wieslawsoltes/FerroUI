//! Port of `LayoutableTests_LayoutRounding.cs` (base unit tests). The tests
//! lay out controls under the test root, so they live with the controls.

use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, ControlImpl, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, IntoRef, Point, Rect, Ref, Size, StyledElementImpl,
    Thickness, VisualImpl,
};
use std::cell::Cell;

/// Declares one test per data row of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

fn measure_adjusts_desired_size_upwards_when_constraint_allows(desired_size: f64, expected_size: f64) {
    let _scope = test_scope();
    let target = TestLayoutable::new(Size::new(desired_size, desired_size), 0.0);
    let root = create_root(1.5, &target, None);

    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(Size::new(expected_size, expected_size), target.desired_size());
}

theory!(measure_adjusts_desired_size_upwards_when_constraint_allows:
    measure_adjusts_desired_size_upwards_when_constraint_allows_1(100.0, 100.0);
    measure_adjusts_desired_size_upwards_when_constraint_allows_2(101.0, 101.33333333333333);
    measure_adjusts_desired_size_upwards_when_constraint_allows_3(103.0, 103.33333333333333));

#[test]
fn measure_constrains_adjusted_desired_size_to_constraint() {
    let _scope = test_scope();
    let target = TestLayoutable::new(Size::new(101.0, 101.0), 0.0);
    let root = create_root(1.5, &target, Some(Size::new(101.0, 101.0)));

    root.layout_manager().execute_initial_layout_pass();

    // Desired width/height with layout rounding is 101.3333 but constraint is 101,101 so
    // layout rounding should be ignored.
    assert_eq!(Size::new(101.0, 101.0), target.desired_size());
}

#[test]
fn measure_adjusts_desired_size_upwards_when_margin_present() {
    let _scope = test_scope();
    let target = TestLayoutable::new(Size::new(101.0, 101.0), 1.0);
    let root = create_root(1.5, &target, None);

    root.layout_manager().execute_initial_layout_pass();

    // - 1 pixel margin is rounded up to 1.3333; for both sides it is 2.6666
    // - Size of 101 gets rounded up to 101.3333
    // - Final size = 101.3333 + 2.6666 = 104
    assert_equal_size(Size::new(104.0, 104.0), target.desired_size());
}

#[test]
fn arrange_adjusts_bounds_upwards_with_margin() {
    let _scope = test_scope();
    let target = TestLayoutable::new(Size::new(101.0, 101.0), 1.0);
    let root = create_root(1.5, &target, None);

    root.layout_manager().execute_initial_layout_pass();

    // - 1 pixel margin is rounded up to 1.3333
    // - Size of 101 gets rounded up to 101.3333
    assert_equal_point(Point::new(1.3333333333333333, 1.3333333333333333), target.bounds().position());
    assert_equal_size(Size::new(101.33333333333333, 101.33333333333333), target.bounds().size());
}

fn arranges_center_alignment_correctly_with_fractional_scaling(
    container_width: f64,
    child_width: f64,
    expected_x: f64,
) {
    let _scope = test_scope();
    let target = Border::new();
    target.set_width(child_width);
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    let decorator = Decorator::new();
    decorator.set_width(container_width);
    decorator.set_height(100.0);
    decorator.set_child(&target);
    let root = TestRoot::new();
    root.set_layout_scaling(1.5);
    root.set_use_layout_rounding(true);
    root.set_child(&decorator);

    root.measure(Size::new(100.0, 100.0));
    root.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Rect::new(expected_x, 0.0, child_width, 100.0), target.bounds());
}

theory!(arranges_center_alignment_correctly_with_fractional_scaling:
    arranges_center_alignment_correctly_with_fractional_scaling_1(16.0, 6.0, 5.333333333333333);
    arranges_center_alignment_correctly_with_fractional_scaling_2(18.0, 10.0, 4.0));

fn create_root(scaling: f64, child: impl IntoRef<Control>, constraint: Option<Size>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_layout_scaling(scaling);
    root.set_use_layout_rounding(true);
    root.set_child(child.into_ref());
    root.set_client_size(constraint.unwrap_or(Size::new(1000.0, 1000.0)));
    root
}

fn assert_equal_point(expected: Point, actual: Point) {
    assert!(expected.nearly_equals(actual), "expected {expected}, actual {actual}");
}

fn assert_equal_size(expected: Size, actual: Size) {
    assert!(expected.nearly_equals(actual), "expected {expected}, actual {actual}");
}

#[repr(C)]
struct TestLayoutable {
    base: Control,
    desired_size: Cell<Size>,
}

ferro_class!(TestLayoutable: Control);
ferro_impl_classes!(
    TestLayoutable: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for TestLayoutable {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        this.desired_size.get()
    }
}

impl TestLayoutable {
    fn new(desired_size: Size, margin: f64) -> Ref<Self> {
        let result = instantiate(Self { base: Control::construct(), desired_size: Cell::new(desired_size) });
        result.set_margin(Thickness::uniform(margin));
        result
    }
}
