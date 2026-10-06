//! Port of `LayoutHelperTests.cs`.

use super::LayoutHelper;

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
