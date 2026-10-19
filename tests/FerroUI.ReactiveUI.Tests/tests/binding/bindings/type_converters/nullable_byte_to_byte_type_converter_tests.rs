//! `NullableByteToByteTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableByteToByteTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_BYTE: u8 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableByteToByteTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_with_value_succeeds() {
    assert_eq!(NullableByteToByteTypeConverter.try_convert(Some(&Some(SAMPLE_BYTE)), None), Some(SAMPLE_BYTE));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(NullableByteToByteTypeConverter.try_convert(None, None), None);
    assert_eq!(NullableByteToByteTypeConverter.try_convert(Some(&None), None), None);
}

#[test]
fn from_type_returns_byte_nullable() {
    assert_eq!(NullableByteToByteTypeConverter.from_type(), ConverterType::of::<Option<u8>>());
}

#[test]
fn to_type_returns_byte() {
    assert_eq!(NullableByteToByteTypeConverter.to_type(), ConverterType::of::<u8>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    // Upstream passes the boxed value; an `Option` holding it converts as well.
    let result = NullableByteToByteTypeConverter.try_convert_typed(Some(&SAMPLE_BYTE), None);
    assert_eq!(converted::<u8>(result), Some(SAMPLE_BYTE));
    let result = NullableByteToByteTypeConverter.try_convert_typed(Some(&Some(SAMPLE_BYTE)), None);
    assert_eq!(converted::<u8>(result), Some(SAMPLE_BYTE));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(NullableByteToByteTypeConverter.try_convert_typed(None, None).is_none());
    assert!(NullableByteToByteTypeConverter.try_convert_typed(Some(&None::<u8>), None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(NullableByteToByteTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
