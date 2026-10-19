//! `BooleanToStringTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/BooleanToStringTypeConverterTests.cs`).

use ferroui_reactiveui::*;

const EXPECTED_AFFINITY: i32 = 2;

#[test]
fn get_affinity_for_objects_returns2() {
    assert_eq!(BooleanToStringTypeConverter.get_affinity_for_objects(), EXPECTED_AFFINITY);
}

#[test]
fn try_convert_true_returns_true() {
    assert_eq!(BooleanToStringTypeConverter.try_convert(Some(&true), None), Some("True".to_string()));
}

#[test]
fn try_convert_false_returns_false() {
    assert_eq!(BooleanToStringTypeConverter.try_convert(Some(&false), None), Some("False".to_string()));
}
