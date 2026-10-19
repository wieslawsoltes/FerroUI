//! `ShortToNullableShortTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/ShortToNullableShortTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_SHORT: i16 = 1_234;
const SMALL_SHORT: i16 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(ShortToNullableShortTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_always_succeeds() {
    let value: i16 = 1_234;
    assert_eq!(ShortToNullableShortTypeConverter.try_convert(Some(&value), None), Some(Some(SAMPLE_SHORT)));
}

#[test]
fn from_type_returns_short() {
    assert_eq!(ShortToNullableShortTypeConverter.from_type(), ConverterType::of::<i16>());
}

#[test]
fn to_type_returns_short_nullable() {
    assert_eq!(ShortToNullableShortTypeConverter.to_type(), ConverterType::of::<Option<i16>>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    let value: i16 = 42;
    let result = ShortToNullableShortTypeConverter.try_convert_typed(Some(&value), None);
    assert_eq!(converted::<Option<i16>>(result), Some(Some(SMALL_SHORT)));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(ShortToNullableShortTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(ShortToNullableShortTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
