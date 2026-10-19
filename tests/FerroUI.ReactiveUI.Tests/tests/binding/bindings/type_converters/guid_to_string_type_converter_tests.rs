//! `GuidToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/GuidToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;
use ferroui_splat::Guid;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(GuidToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_guid_succeeds() {
    // `Guid.Parse("12345678-1234-1234-1234-123456789abc")`.
    let value = Guid::from_u128(0x12345678_1234_1234_1234_123456789abc);
    assert_eq!(
        GuidToStringTypeConverter.try_convert(Some(&value), None),
        Some("12345678-1234-1234-1234-123456789abc".to_string())
    );
}

#[test]
fn try_convert_empty_guid_succeeds() {
    assert_eq!(
        GuidToStringTypeConverter.try_convert(Some(&Guid::EMPTY), None),
        Some("00000000-0000-0000-0000-000000000000".to_string())
    );
}
