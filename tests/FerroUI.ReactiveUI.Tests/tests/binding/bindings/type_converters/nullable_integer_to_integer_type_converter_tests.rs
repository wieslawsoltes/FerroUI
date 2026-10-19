//! `NullableIntegerToIntegerTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableIntegerToIntegerTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_INTEGER: i32 = 123_456;
const SMALL_INTEGER: i32 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableIntegerToIntegerTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_with_value_succeeds() {
    assert_eq!(NullableIntegerToIntegerTypeConverter.try_convert(Some(&Some(SAMPLE_INTEGER)), None), Some(SAMPLE_INTEGER));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(NullableIntegerToIntegerTypeConverter.try_convert(None, None), None);
    assert_eq!(NullableIntegerToIntegerTypeConverter.try_convert(Some(&None), None), None);
}

#[test]
fn from_type_returns_int_nullable() {
    assert_eq!(NullableIntegerToIntegerTypeConverter.from_type(), ConverterType::of::<Option<i32>>());
}

#[test]
fn to_type_returns_int() {
    assert_eq!(NullableIntegerToIntegerTypeConverter.to_type(), ConverterType::of::<i32>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    // Upstream passes the boxed value; an `Option` holding it converts as well.
    let result = NullableIntegerToIntegerTypeConverter.try_convert_typed(Some(&SMALL_INTEGER), None);
    assert_eq!(converted::<i32>(result), Some(SMALL_INTEGER));
    let result = NullableIntegerToIntegerTypeConverter.try_convert_typed(Some(&Some(SMALL_INTEGER)), None);
    assert_eq!(converted::<i32>(result), Some(SMALL_INTEGER));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(NullableIntegerToIntegerTypeConverter.try_convert_typed(None, None).is_none());
    assert!(NullableIntegerToIntegerTypeConverter.try_convert_typed(Some(&None::<i32>), None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(NullableIntegerToIntegerTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
