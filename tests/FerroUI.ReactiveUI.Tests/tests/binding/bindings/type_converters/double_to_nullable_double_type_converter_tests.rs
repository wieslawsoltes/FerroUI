//! `DoubleToNullableDoubleTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/DoubleToNullableDoubleTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;
const SAMPLE_DOUBLE: f64 = 123.456789;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(DoubleToNullableDoubleTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_always_succeeds() {
    let value: f64 = 123.456789;
    assert_eq!(DoubleToNullableDoubleTypeConverter.try_convert(Some(&value), None), Some(Some(SAMPLE_DOUBLE)));
}

#[test]
fn from_type_returns_double() {
    assert_eq!(DoubleToNullableDoubleTypeConverter.from_type(), ConverterType::of::<f64>());
}

#[test]
fn to_type_returns_double_nullable() {
    assert_eq!(DoubleToNullableDoubleTypeConverter.to_type(), ConverterType::of::<Option<f64>>());
}
