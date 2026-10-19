//! `NullableGuidToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableGuidToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use ferroui_splat::Guid;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableGuidToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_guid_succeeds() {
    let value = Some(Guid::from_u128(0x12345678_1234_1234_1234_123456789abc));
    assert_eq!(
        NullableGuidToStringTypeConverter.try_convert(Some(&value), None),
        Some("12345678-1234-1234-1234-123456789abc".to_string())
    );
}

#[test]
fn try_convert_null_returns_null_string() {
    // Upstream converts null to a null string; the port converts it to an empty string.
    assert_eq!(NullableGuidToStringTypeConverter.try_convert(None, None), Some(String::new()));
}
