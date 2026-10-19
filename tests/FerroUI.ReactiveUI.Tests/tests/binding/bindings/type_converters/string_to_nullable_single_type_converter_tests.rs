//! `StringToNullableSingleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/StringToNullableSingleTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const PARSED_SINGLE: f32 = 123.456;
const SINGLE_TOLERANCE: f32 = 0.001;

fn convert(value: &str) -> Option<Option<f32>> {
    StringToNullableSingleTypeConverter.try_convert(Some(&value.to_string()), None)
}

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(StringToNullableSingleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_empty_string_returns_true() {
    assert_eq!(convert(""), Some(None));
}

#[test]
fn try_convert_invalid_string_returns_false() {
    assert_eq!(convert("invalid"), None);
}

#[test]
fn try_convert_string_to_single_nullable_succeeds() {
    let output = convert("123.456").expect("converted").expect("a value");
    super::assert_within(output, PARSED_SINGLE, SINGLE_TOLERANCE);
}
