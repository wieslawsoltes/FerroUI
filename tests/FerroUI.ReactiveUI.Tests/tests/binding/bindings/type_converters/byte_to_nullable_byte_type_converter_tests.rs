//! `ByteToNullableByteTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/ByteToNullableByteTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_BYTE: u8 = 42;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(ByteToNullableByteTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_always_succeeds() {
    let value: u8 = 42;
    assert_eq!(ByteToNullableByteTypeConverter.try_convert(Some(&value), None), Some(Some(SAMPLE_BYTE)));
}

#[test]
fn from_type_returns_byte() {
    assert_eq!(ByteToNullableByteTypeConverter.from_type(), ConverterType::of::<u8>());
}

#[test]
fn to_type_returns_byte_nullable() {
    assert_eq!(ByteToNullableByteTypeConverter.to_type(), ConverterType::of::<Option<u8>>());
}

#[test]
fn try_convert_typed_with_valid_value_returns_true_and_output() {
    let value: u8 = 42;
    let result = ByteToNullableByteTypeConverter.try_convert_typed(Some(&value), None);
    assert_eq!(converted::<Option<u8>>(result), Some(Some(SAMPLE_BYTE)));
}

#[test]
fn try_convert_typed_with_null_value_returns_false() {
    assert!(ByteToNullableByteTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_with_invalid_type_returns_false() {
    let value = String::from("invalid");
    assert!(ByteToNullableByteTypeConverter.try_convert_typed(Some(&value), None).is_none());
}
