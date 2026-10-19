//! `StringToGuidTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToGuidTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use ferroui_splat::Guid;

const EXPECTED_AFFINITY: i32 = 2;

fn convert(value: &str) -> Option<Guid> {
    StringToGuidTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToGuidTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_valid_string_succeeds() {
    let expected = Guid::from_u128(0x12345678_1234_1234_1234_123456789abc);
    assert_eq!(convert("12345678-1234-1234-1234-123456789abc"), Some(expected));
}

#[test]
fn try_convert_empty_guid_string_succeeds() {
    assert_eq!(convert("00000000-0000-0000-0000-000000000000"), Some(Guid::EMPTY));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(StringToGuidTypeConverter.try_convert(None, None), None);
}

#[test]
fn try_convert_empty_string_returns_false() {
    assert_eq!(convert(""), None);
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(convert("invalid"), None);
}

/// The other formats of `Guid.TryParse`, which upstream's tests do not reach.
#[test]
fn try_convert_other_formats() {
    let expected = Some(Guid::from_u128(0x12345678_1234_1234_1234_123456789abc));
    assert_eq!(convert("12345678123412341234123456789abc"), expected);
    assert_eq!(convert("{12345678-1234-1234-1234-123456789abc}"), expected);
    assert_eq!(convert("(12345678-1234-1234-1234-123456789ABC)"), expected);
    assert_eq!(convert("{0x12345678,0x1234,0x1234,{0x12,0x34,0x12,0x34,0x56,0x78,0x9a,0xbc}}"), expected);
    assert_eq!(convert("{12345678-1234-1234-1234-123456789abc)"), None);
}
