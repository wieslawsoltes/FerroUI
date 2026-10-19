//! `NullableShortToShortTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableShortToShortTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_SHORT: i16 = 1_234;
const SMALL_SHORT: i16 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableShortToShortTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_with_value_succeeds() {
    assert_eq!(NullableShortToShortTypeConverter.try_convert(Some(&Some(SAMPLE_SHORT)), None), Some(SAMPLE_SHORT));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(NullableShortToShortTypeConverter.try_convert(None, None), None);
    assert_eq!(NullableShortToShortTypeConverter.try_convert(Some(&None), None), None);
}

#[test]
fn from_type_returns_short_nullable() {
    assert_eq!(NullableShortToShortTypeConverter.from_type(), ConverterType::of::<Option<i16>>());
}

#[test]
fn to_type_returns_short() {
    assert_eq!(NullableShortToShortTypeConverter.to_type(), ConverterType::of::<i16>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    // Upstream passes the boxed value; an `Option` holding it converts as well.
    let result = NullableShortToShortTypeConverter.try_convert_typed(Some(&SMALL_SHORT), None);
    assert_eq!(converted::<i16>(result), Some(SMALL_SHORT));
    let result = NullableShortToShortTypeConverter.try_convert_typed(Some(&Some(SMALL_SHORT)), None);
    assert_eq!(converted::<i16>(result), Some(SMALL_SHORT));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(NullableShortToShortTypeConverter.try_convert_typed(None, None).is_none());
    assert!(NullableShortToShortTypeConverter.try_convert_typed(Some(&None::<i16>), None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(NullableShortToShortTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
