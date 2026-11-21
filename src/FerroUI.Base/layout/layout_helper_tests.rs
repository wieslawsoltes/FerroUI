//! Port of `LayoutHelperTests.cs`.

use super::LayoutHelper;

#[test]
fn winui_measure_constraint_rounding_follows_the_inherited_layout_option() {
    use super::{Layoutable, LayoutableImpl};
    use crate::threading::Dispatcher;
    use crate::tree_tests::{TestRoot, TestSource};
    use crate::{FerroObjectImpl, Rect, Size, StyledElementImpl, Thickness, VisualImpl, ferro_class, ferro_impl_classes, instantiate};
    use std::cell::Cell;

    #[repr(C)]
    struct ConstraintRecorder {
        base: Layoutable,
        available: Cell<Size>,
    }

    ferro_class!(ConstraintRecorder: Layoutable);
    ferro_impl_classes!(ConstraintRecorder: FerroObjectImpl, StyledElementImpl, VisualImpl);
    impl LayoutableImpl for ConstraintRecorder {
        fn measure_override(this: &Self, available: Size) -> Size {
            this.available.set(available);
            Size::new(0.0, 73.2)
        }
    }

    let _scope = Dispatcher::unit_test_scope();
    let root = TestRoot::new();
    let child = instantiate(ConstraintRecorder {
        base: Layoutable::construct(),
        available: Cell::new(Size::default()),
    });
    root.logical_children().add(child.clone().upcast());
    root.visual_children().add(child.clone().upcast());
    let source = TestSource::new(root.clone(), 1.25);
    root.set_presentation_source_for_root_visual(Some(source));

    let available = Size::new(781.6, f64::INFINITY);
    child.measure(available);
    assert_eq!(child.available.get(), available);
    assert_eq!(child.desired_size().height, 73.6);

    root.set_use_winui_measure_rounding(true);
    child.measure(available);
    assert_eq!(child.available.get(), Size::new(f64::from(780.8_f32), f64::INFINITY));
    child.measure(Size::new(f64::from(780.8_f32), f64::INFINITY));
    assert_eq!(child.available.get(), Size::new(780.0, f64::INFINITY));
    // The source treats 73.2 DIP as a float before rounding at 125%.
    assert_eq!(child.desired_size().height, 72.8);
    let padded = LayoutHelper::measure_child_with_border(
        Some(&child), Size::INFINITY, Thickness::new(4.0, 3.0, 4.0, 3.0), Thickness::default(),
    );
    assert_eq!(padded.height, 78.8);
    let decorated = LayoutHelper::measure_child(
        Some(&child), Size::INFINITY, Thickness::new(4.0, 3.0, 4.0, 3.0),
    );
    assert_eq!(decorated.height, 78.8, "padding keeps its authored size on the decorator path");
    child.set_height(17.0);
    child.measure(available);
    assert_eq!(child.available.get().height, 16.8);
    assert_eq!(child.desired_size().height, 16.8);
    child.arrange(Rect::new(0.0, 45.2, 20.0, 16.8));
    assert_eq!(child.bounds().height, 16.8);
    assert_eq!(child.bounds().y, 45.6);

    child.set_height(f64::NAN);
    child.set_use_layout_rounding(false);
    child.measure(available);
    assert_eq!(child.available.get(), available);
}

#[test]
fn round_layout_value_without_dpi_aware() {
    const VALUE: f64 = 42.5;
    let expected_value = VALUE.round_ties_even();
    let actual_value = LayoutHelper::round_layout_value(VALUE, 1.0);
    assert_eq!(expected_value, actual_value);
}

#[test]
fn round_layout_value_with_dpi_aware() {
    const DPI_SCALE: f64 = 1.25;
    const VALUE: f64 = 42.5;
    let expected_value = (VALUE * DPI_SCALE).round_ties_even() / DPI_SCALE;
    let actual_value = LayoutHelper::round_layout_value(VALUE, DPI_SCALE);
    assert_eq!(expected_value, actual_value);
}

#[test]
fn validate_scaling_returns_exact_one_for_approximate_one() {
    let result = LayoutHelper::validate_scaling(1.000000000000001);
    assert_eq!(1.0, result);
}

#[test]
fn validate_scaling_returns_valid_scaling_value() {
    const SCALING: f64 = 1.5;
    let result = LayoutHelper::validate_scaling(SCALING);
    assert_eq!(SCALING, result);
}

fn validate_scaling_throws_for_invalid_values(scaling: f64) {
    LayoutHelper::validate_scaling(scaling);
}

#[test]
#[should_panic(expected = "Invalid render scaling value")]
fn validate_scaling_throws_for_invalid_values_1() {
    validate_scaling_throws_for_invalid_values(0.0);
}

#[test]
#[should_panic(expected = "Invalid render scaling value")]
fn validate_scaling_throws_for_invalid_values_2() {
    validate_scaling_throws_for_invalid_values(-1.5);
}

#[test]
#[should_panic(expected = "Invalid render scaling value")]
fn validate_scaling_throws_for_invalid_values_3() {
    validate_scaling_throws_for_invalid_values(f64::NAN);
}

#[test]
#[should_panic(expected = "Invalid render scaling value")]
fn validate_scaling_throws_for_invalid_values_4() {
    validate_scaling_throws_for_invalid_values(f64::INFINITY);
}

#[test]
#[should_panic(expected = "Invalid render scaling value")]
fn validate_scaling_throws_for_invalid_values_5() {
    validate_scaling_throws_for_invalid_values(f64::NEG_INFINITY);
}
