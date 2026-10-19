//! `IntegerToNullableIntegerTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/IntegerToNullableIntegerTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_INTEGER: i32 = 123_456;
const SMALL_INTEGER: i32 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(IntegerToNullableIntegerTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_always_succeeds() {
    let value: i32 = 123_456;
    assert_eq!(IntegerToNullableIntegerTypeConverter.try_convert(Some(&value), None), Some(Some(SAMPLE_INTEGER)));
}

#[test]
fn from_type_returns_int() {
    assert_eq!(IntegerToNullableIntegerTypeConverter.from_type(), ConverterType::of::<i32>());
}

#[test]
fn to_type_returns_int_nullable() {
    assert_eq!(IntegerToNullableIntegerTypeConverter.to_type(), ConverterType::of::<Option<i32>>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    let value: i32 = 42;
    let result = IntegerToNullableIntegerTypeConverter.try_convert_typed(Some(&value), None);
    assert_eq!(converted::<Option<i32>>(result), Some(Some(SMALL_INTEGER)));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(IntegerToNullableIntegerTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(IntegerToNullableIntegerTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
