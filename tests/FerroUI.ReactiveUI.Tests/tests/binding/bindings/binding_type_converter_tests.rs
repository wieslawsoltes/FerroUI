//! `BindingTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/BindingTypeConverterTests.cs`).

use super::type_converters::converted;
use ferroui_reactiveui::*;
use std::any::Any;

const SOURCE_VALUE: f64 = 123.45;
const PARSED_INTEGER: i32 = 42;

fn text(value: &str) -> String {
    value.to_string()
}

#[test]
fn from_type_returns_correct_type() {
    assert_eq!(TestConverter.from_type(), ConverterType::of::<String>());
}

#[test]
fn to_type_returns_correct_type() {
    assert_eq!(TestConverter.to_type(), ConverterType::of::<i32>());
}

#[test]
fn try_convert_typed_type_mismatch_returns_false() {
    assert!(TestConverter.try_convert_typed(Some(&SOURCE_VALUE), None).is_none());
}

#[test]
fn try_convert_typed_valid_input_succeeds() {
    assert_eq!(converted::<i32>(TestConverter.try_convert_typed(Some(&text("42")), None)), Some(PARSED_INTEGER));
}

#[test]
fn try_convert_typed_conversion_fails_returns_false() {
    assert!(TestConverter.try_convert_typed(Some(&text("invalid")), None).is_none());
}

#[test]
fn try_convert_typed_null_input_with_reference_type_source_handles_correctly() {
    assert!(TestConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_null_input_with_value_type_source_returns_false() {
    // The base hands `None` to the conversion, and a converter of a value type fails on it.
    assert!(ValueTypeConverter.try_convert_typed(None, None).is_none());
}

#[test]
fn try_convert_typed_null_input_with_nullable_source_succeeds() {
    assert_eq!(converted::<String>(NullableToStringConverter.try_convert_typed(None, None)), Some("null".to_string()));
}

#[test]
fn try_convert_typed_null_result_with_non_nullable_target_returns_false() {
    assert!(NullReturningConverter.try_convert_typed(Some(&text("test")), None).is_none());
}

#[test]
fn try_convert_typed_null_result_with_nullable_target_succeeds() {
    let result = StringToNullableIntConverter.try_convert_typed(Some(&text("null")), None);
    assert_eq!(converted::<Option<i32>>(result), Some(None));
}

#[test]
fn try_convert_typed_null_input_with_nullable_source_but_conversion_fails_returns_false() {
    assert!(NullableFailingConverter.try_convert_typed(None, None).is_none());
}

/// Parses a string to an `i32` (`TestConverter`).
struct TestConverter;

impl BindingTypeConverter for TestConverter {
    type From = String;
    type To = i32;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<i32> {
        from?.parse().ok()
    }
}

/// Writes an `i32` (`ValueTypeConverter`).
struct ValueTypeConverter;

impl BindingTypeConverter for ValueTypeConverter {
    type From = i32;
    type To = String;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&i32>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        Some(from?.to_string())
    }
}

/// Writes an `Option<i32>`, `None` as `null` (`NullableToStringConverter`).
struct NullableToStringConverter;

impl BindingTypeConverter for NullableToStringConverter {
    type From = Option<i32>;
    type To = String;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&Option<i32>>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        Some(from.copied().flatten().map_or_else(|| "null".to_string(), |value| value.to_string()))
    }
}

/// Always fails (`NullReturningConverter`).
struct NullReturningConverter;

impl BindingTypeConverter for NullReturningConverter {
    type From = String;
    type To = i32;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, _from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<i32> {
        None
    }
}

/// Parses a string to an `Option<i32>`, `null` to `None` (`StringToNullableIntConverter`).
struct StringToNullableIntConverter;

impl BindingTypeConverter for StringToNullableIntConverter {
    type From = String;
    type To = Option<i32>;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<Option<i32>> {
        let from = from?;
        if from == "null" {
            return Some(None);
        }
        from.parse().ok().map(Some)
    }
}

/// Writes an `Option<i32>` and fails on `None` (`NullableFailingConverter`).
struct NullableFailingConverter;

impl BindingTypeConverter for NullableFailingConverter {
    type From = Option<i32>;
    type To = String;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&Option<i32>>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        from.copied().flatten().map(|value| value.to_string())
    }
}
