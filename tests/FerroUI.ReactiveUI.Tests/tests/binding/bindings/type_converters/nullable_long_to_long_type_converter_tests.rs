//! `NullableLongToLongTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableLongToLongTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_LONG: i64 = 1_234_567_890_123_456;
const SMALL_LONG: i64 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableLongToLongTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_with_value_succeeds() {
    assert_eq!(NullableLongToLongTypeConverter.try_convert(Some(&Some(SAMPLE_LONG)), None), Some(SAMPLE_LONG));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(NullableLongToLongTypeConverter.try_convert(None, None), None);
    assert_eq!(NullableLongToLongTypeConverter.try_convert(Some(&None), None), None);
}

#[test]
fn from_type_returns_long_nullable() {
    assert_eq!(NullableLongToLongTypeConverter.from_type(), ConverterType::of::<Option<i64>>());
}

#[test]
fn to_type_returns_long() {
    assert_eq!(NullableLongToLongTypeConverter.to_type(), ConverterType::of::<i64>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    // Upstream passes the boxed value; an `Option` holding it converts as well.
    let result = NullableLongToLongTypeConverter.try_convert_typed(Some(&SMALL_LONG), None);
    assert_eq!(converted::<i64>(result), Some(SMALL_LONG));
    let result = NullableLongToLongTypeConverter.try_convert_typed(Some(&Some(SMALL_LONG)), None);
    assert_eq!(converted::<i64>(result), Some(SMALL_LONG));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(NullableLongToLongTypeConverter.try_convert_typed(None, None).is_none());
    assert!(NullableLongToLongTypeConverter.try_convert_typed(Some(&None::<i64>), None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(NullableLongToLongTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
