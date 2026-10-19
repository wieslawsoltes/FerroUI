//! `BindingTypeConverterBaseTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/BindingTypeConverterBaseTests.cs`).
//!
//! The base runs a converter's typed conversion from the type-erased one. Upstream's `null` input
//! is `None`; a `null` result of a nullable target is the `None` of an `Option`, and a `null`
//! string is an empty string.

use super::converted;
use ferroui_reactiveui::*;
use std::any::Any;

const CONVERSION_HINT: i32 = 3;
const SOURCE_VALUE: i32 = 5;
const PARSED_INTEGER: i32 = 42;
const BOXED_INTEGER: i32 = 100;

fn text(value: &str) -> String {
    value.to_string()
}

#[test]
fn try_convert_typed_null_input_non_nullable_value_type_from_returns_false() {
    assert!(IntegerToStringTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_wrong_input_type_returns_false() {
    assert!(StringToIntegerTypeConverter.try_convert_typed(Some(&PARSED_INTEGER), None).is_none());
}

#[test]
fn try_convert_typed_conversion_fails_returns_false() {
    assert!(StringToIntegerTypeConverter.try_convert_typed(Some(&text("not-a-number")), None).is_none());
}

#[test]
fn try_convert_typed_valid_input_returns_true() {
    let result = StringToIntegerTypeConverter.try_convert_typed(Some(&text("42")), None);
    assert_eq!(converted::<i32>(result), Some(PARSED_INTEGER));
}

#[test]
fn try_convert_typed_value_type_to_reference_type_succeeds() {
    let result = IntegerToStringTypeConverter.try_convert_typed(Some(&PARSED_INTEGER), None);
    assert_eq!(converted::<String>(result), Some("42".to_string()));
}

#[test]
fn try_convert_typed_null_input_nullable_source_try_convert_fails_returns_false() {
    assert!(NullRejectingNullableConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_null_input_nullable_source_converts_to_nullable_result_succeeds() {
    // Upstream converts null to a null string; the port converts it to an empty string.
    let result = NullableBooleanToStringTypeConverter.try_convert_typed(None, None);
    assert_eq!(converted::<String>(result), Some(String::new()));
}

#[test]
fn try_convert_typed_null_input_reference_type_source_converts_to_non_null_result_succeeds() {
    let result = NullAcceptingStringToStringConverter.try_convert_typed(None, None);
    assert_eq!(converted::<String>(result), Some("(null)".to_string()));
}

#[test]
fn try_convert_typed_null_input_null_result_with_nullable_target_succeeds() {
    let result = NullToNullStringConverter.try_convert_typed(None, None);
    assert_eq!(converted::<Option<String>>(result), Some(None));
}

#[test]
fn try_convert_typed_null_input_reference_type_source_try_convert_fails_returns_false() {
    assert!(StringToIntegerTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_with_conversion_hint_passes_through_to_try_convert() {
    let result = IntegerToStringTypeConverter.try_convert_typed(Some(&SOURCE_VALUE), Some(&CONVERSION_HINT));
    assert_eq!(converted::<String>(result), Some("005".to_string()));
}

#[test]
fn try_convert_typed_value_type_result_is_boxed_correctly() {
    let result = StringToIntegerTypeConverter.try_convert_typed(Some(&text("100")), None).expect("converted");
    assert!(result.is::<i32>());
    assert_eq!(result.downcast_ref::<i32>(), Some(&BOXED_INTEGER));
}

#[test]
fn from_type_returns_correct_generic_type() {
    assert_eq!(NullAcceptingStringToStringConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_correct_generic_type() {
    assert_eq!(NullAcceptingStringToStringConverter.to_type(), ConverterType::of::<String>());
}

#[test]
fn try_convert_typed_nullable_value_type_target_null_input_returns_null_result() {
    let result = StringToNullableIntegerTypeConverter.try_convert_typed(None, None);
    assert_eq!(converted::<Option<i32>>(result), Some(None));
}

#[test]
fn try_convert_typed_nullable_value_type_target_valid_input_converts_successfully() {
    let result = StringToNullableIntegerTypeConverter.try_convert_typed(Some(&text("42")), None);
    assert_eq!(converted::<Option<i32>>(result), Some(Some(PARSED_INTEGER)));
}

#[test]
fn nullable_byte_to_byte_type_converter_try_convert_typed_wrong_type_returns_false() {
    assert!(NullableByteToByteTypeConverter.try_convert_typed(Some(&"not a byte"), None).is_none());
}

// Not ported: NullableDecimalToDecimalTypeConverter_TryConvertTyped_WrongType_ReturnsFalse - needs Decimal in ferroui-system (phase 1b)

#[test]
fn nullable_double_to_double_type_converter_try_convert_typed_wrong_type_returns_false() {
    assert!(NullableDoubleToDoubleTypeConverter.try_convert_typed(Some(&"not a double"), None).is_none());
}

#[test]
fn nullable_integer_to_integer_type_converter_try_convert_typed_wrong_type_returns_false() {
    assert!(NullableIntegerToIntegerTypeConverter.try_convert_typed(Some(&"not an int"), None).is_none());
}

#[test]
fn nullable_long_to_long_type_converter_try_convert_typed_wrong_type_returns_false() {
    assert!(NullableLongToLongTypeConverter.try_convert_typed(Some(&"not a long"), None).is_none());
}

#[test]
fn nullable_short_to_short_type_converter_try_convert_typed_wrong_type_returns_false() {
    assert!(NullableShortToShortTypeConverter.try_convert_typed(Some(&"not a short"), None).is_none());
}

#[test]
fn nullable_single_to_single_type_converter_try_convert_typed_wrong_type_returns_false() {
    assert!(NullableSingleToSingleTypeConverter.try_convert_typed(Some(&"not a float"), None).is_none());
}

/// A converter from `Option<i32>` that fails on `None` (`NullRejectingNullableConverter`).
struct NullRejectingNullableConverter;

impl BindingTypeConverter for NullRejectingNullableConverter {
    type From = Option<i32>;
    type To = String;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&Option<i32>>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        from.copied().flatten().map(|value| value.to_string())
    }
}

/// A converter of strings that converts no string to `(null)`
/// (`NullAcceptingStringToStringConverter`).
struct NullAcceptingStringToStringConverter;

impl BindingTypeConverter for NullAcceptingStringToStringConverter {
    type From = String;
    type To = String;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        Some(from.cloned().unwrap_or_else(|| "(null)".to_string()))
    }
}

/// A converter that converts a string to itself and no string to no string
/// (`NullToNullStringConverter`); a string that may be null is an `Option<String>` here.
struct NullToNullStringConverter;

impl BindingTypeConverter for NullToNullStringConverter {
    type From = String;
    type To = Option<String>;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<String>> {
        Some(from.cloned())
    }
}
