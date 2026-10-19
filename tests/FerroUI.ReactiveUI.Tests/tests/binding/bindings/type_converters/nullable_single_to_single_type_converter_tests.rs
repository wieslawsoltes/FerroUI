//! `NullableSingleToSingleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableSingleToSingleTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_SINGLE: f32 = 123.45;
const ROUNDED_SINGLE: f32 = 42.5;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableSingleToSingleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_with_value_succeeds() {
    assert_eq!(NullableSingleToSingleTypeConverter.try_convert(Some(&Some(SAMPLE_SINGLE)), None), Some(SAMPLE_SINGLE));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(NullableSingleToSingleTypeConverter.try_convert(None, None), None);
    assert_eq!(NullableSingleToSingleTypeConverter.try_convert(Some(&None), None), None);
}

#[test]
fn from_type_returns_float_nullable() {
    assert_eq!(NullableSingleToSingleTypeConverter.from_type(), ConverterType::of::<Option<f32>>());
}

#[test]
fn to_type_returns_single() {
    assert_eq!(NullableSingleToSingleTypeConverter.to_type(), ConverterType::of::<f32>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    // Upstream passes the boxed value; an `Option` holding it converts as well.
    let result = NullableSingleToSingleTypeConverter.try_convert_typed(Some(&ROUNDED_SINGLE), None);
    assert_eq!(converted::<f32>(result), Some(ROUNDED_SINGLE));
    let result = NullableSingleToSingleTypeConverter.try_convert_typed(Some(&Some(ROUNDED_SINGLE)), None);
    assert_eq!(converted::<f32>(result), Some(ROUNDED_SINGLE));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(NullableSingleToSingleTypeConverter.try_convert_typed(None, None).is_none());
    assert!(NullableSingleToSingleTypeConverter.try_convert_typed(Some(&None::<f32>), None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(NullableSingleToSingleTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
