//! `LongToNullableLongTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/LongToNullableLongTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_LONG: i64 = 1_234_567_890_123_456;
const SMALL_LONG: i64 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(LongToNullableLongTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_always_succeeds() {
    let value: i64 = 1_234_567_890_123_456;
    assert_eq!(LongToNullableLongTypeConverter.try_convert(Some(&value), None), Some(Some(SAMPLE_LONG)));
}

#[test]
fn from_type_returns_long() {
    assert_eq!(LongToNullableLongTypeConverter.from_type(), ConverterType::of::<i64>());
}

#[test]
fn to_type_returns_long_nullable() {
    assert_eq!(LongToNullableLongTypeConverter.to_type(), ConverterType::of::<Option<i64>>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    let value: i64 = 42;
    let result = LongToNullableLongTypeConverter.try_convert_typed(Some(&value), None);
    assert_eq!(converted::<Option<i64>>(result), Some(Some(SMALL_LONG)));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(LongToNullableLongTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(LongToNullableLongTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
