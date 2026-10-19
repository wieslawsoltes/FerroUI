//! `StringToNullableGuidTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableGuidTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use ferroui_splat::Guid;

const EXPECTED_AFFINITY: i32 = 2;

fn convert(value: &str) -> Option<Option<Guid>> {
    StringToNullableGuidTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableGuidTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_valid_string_succeeds() {
    let expected = Guid::from_u128(0x12345678_1234_1234_1234_123456789abc);
    assert_eq!(convert("12345678-1234-1234-1234-123456789abc"), Some(Some(expected)));
}

#[test]
fn try_convert_null_returns_null() {
    assert_eq!(StringToNullableGuidTypeConverter.try_convert(None, None), Some(None));
}

#[test]
fn try_convert_empty_string_returns_null() {
    assert_eq!(convert(""), Some(None));
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(convert("invalid"), None);
}
