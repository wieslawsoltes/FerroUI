//! `BindingTypeConvertersUnitTests` (`ReactiveUI.Binding.Tests/Bindings/BindingTypeConvertersUnitTests.cs`).

use ferroui_reactiveui::*;

#[test]
fn byte_to_string_type_converter_converts_correctly() {
    assert_eq!(ByteToStringTypeConverter.try_convert(Some(&123), None), Some("123".to_string()));
}

// Not ported: DecimalToStringTypeConverter_Converts_Correctly - needs Decimal in ferroui-system (phase 1b)

#[test]
fn double_to_string_type_converter_converts_correctly() {
    assert_eq!(DoubleToStringTypeConverter.try_convert(Some(&123.456789), None), Some("123.456789".to_string()));
}

#[test]
fn integer_to_string_type_converter_converts_correctly() {
    assert_eq!(IntegerToStringTypeConverter.try_convert(Some(&123_456_789), None), Some("123456789".to_string()));
}

#[test]
fn long_to_string_type_converter_converts_correctly() {
    assert_eq!(
        LongToStringTypeConverter.try_convert(Some(&1_234_567_890_123_456_789), None),
        Some("1234567890123456789".to_string())
    );
}

#[test]
fn nullable_byte_to_string_type_converter_converts_correctly() {
    assert_eq!(NullableByteToStringTypeConverter.try_convert(Some(&Some(123)), None), Some("123".to_string()));
}

// Not ported: NullableDecimalToStringTypeConverter_Converts_Correctly - needs Decimal in ferroui-system (phase 1b)

#[test]
fn nullable_double_to_string_type_converter_converts_correctly() {
    assert_eq!(
        NullableDoubleToStringTypeConverter.try_convert(Some(&Some(123.456789)), None),
        Some("123.456789".to_string())
    );
}

#[test]
fn nullable_integer_to_string_type_converter_converts_correctly() {
    assert_eq!(
        NullableIntegerToStringTypeConverter.try_convert(Some(&Some(123_456_789)), None),
        Some("123456789".to_string())
    );
}

#[test]
fn nullable_long_to_string_type_converter_converts_correctly() {
    assert_eq!(
        NullableLongToStringTypeConverter.try_convert(Some(&Some(1_234_567_890_123_456_789)), None),
        Some("1234567890123456789".to_string())
    );
}

#[test]
fn nullable_short_to_string_type_converter_converts_correctly() {
    assert_eq!(NullableShortToStringTypeConverter.try_convert(Some(&Some(12_345)), None), Some("12345".to_string()));
}

#[test]
fn nullable_single_to_string_type_converter_converts_correctly() {
    assert_eq!(NullableSingleToStringTypeConverter.try_convert(Some(&Some(123.45)), None), Some("123.45".to_string()));
}

#[test]
fn short_to_string_type_converter_converts_correctly() {
    assert_eq!(ShortToStringTypeConverter.try_convert(Some(&12_345), None), Some("12345".to_string()));
}

#[test]
fn single_to_string_type_converter_converts_correctly() {
    assert_eq!(SingleToStringTypeConverter.try_convert(Some(&123.45), None), Some("123.45".to_string()));
}
