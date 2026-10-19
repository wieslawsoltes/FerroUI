//! `SingleToNullableSingleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/SingleToNullableSingleTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_SINGLE: f32 = 123.45;
const ROUNDED_SINGLE: f32 = 42.5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(SingleToNullableSingleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_always_succeeds() {
    let value: f32 = 123.45;
    assert_eq!(SingleToNullableSingleTypeConverter.try_convert(Some(&value), None), Some(Some(SAMPLE_SINGLE)));
}

#[test]
fn from_type_returns_float() {
    assert_eq!(SingleToNullableSingleTypeConverter.from_type(), ConverterType::of::<f32>());
}

#[test]
fn to_type_returns_single_nullable() {
    assert_eq!(SingleToNullableSingleTypeConverter.to_type(), ConverterType::of::<Option<f32>>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    let value: f32 = 42.5;
    let result = SingleToNullableSingleTypeConverter.try_convert_typed(Some(&value), None);
    assert_eq!(converted::<Option<f32>>(result), Some(Some(ROUNDED_SINGLE)));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(SingleToNullableSingleTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(SingleToNullableSingleTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
