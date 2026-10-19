//! `NullableDoubleToDoubleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/NullableDoubleToDoubleTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_DOUBLE: f64 = 123.456789;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(NullableDoubleToDoubleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_with_value_succeeds() {
    assert_eq!(NullableDoubleToDoubleTypeConverter.try_convert(Some(&Some(SAMPLE_DOUBLE)), None), Some(SAMPLE_DOUBLE));
}

#[test]
fn try_convert_null_returns_false() {
    assert_eq!(NullableDoubleToDoubleTypeConverter.try_convert(None, None), None);
    assert_eq!(NullableDoubleToDoubleTypeConverter.try_convert(Some(&None), None), None);
}

#[test]
fn from_type_returns_double_nullable() {
    assert_eq!(NullableDoubleToDoubleTypeConverter.from_type(), ConverterType::of::<Option<f64>>());
}

#[test]
fn to_type_returns_double() {
    assert_eq!(NullableDoubleToDoubleTypeConverter.to_type(), ConverterType::of::<f64>());
}
