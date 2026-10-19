//! `EqualityTypeConverterTests` (`ReactiveUI.Binding.Tests/Bindings/TypeConverters/EqualityTypeConverterTests.cs`).

use super::converted;
use ferroui_reactiveui::*;
use std::any::Any;

const DIFFERENT_VALUE: i32 = 43;
const SAMPLE_VALUE: i32 = 42;

/// The converted `bool` of a conversion, which always succeeds.
fn equal(from: Option<&dyn Any>, hint: Option<&dyn Any>) -> bool {
    converted::<bool>(EqualityTypeConverter.try_convert_typed(from, hint)).expect("the converter always succeeds")
}

#[test]
fn from_type_returns_object_type() {
    assert_eq!(EqualityTypeConverter.from_type(), ConverterType::of::<dyn Any>());
}

#[test]
fn get_affinity_for_objects_returns1() {
    assert_eq!(EqualityTypeConverter.get_affinity_for_objects(), 1);
}

#[test]
fn to_type_returns_bool_type() {
    assert_eq!(EqualityTypeConverter.to_type(), ConverterType::of::<bool>());
}

#[test]
fn try_convert_typed_both_null_returns_true() {
    assert!(equal(None, None));
}

#[test]
fn try_convert_typed_different_integers_returns_false() {
    assert!(!equal(Some(&42), Some(&DIFFERENT_VALUE)));
}

#[test]
fn try_convert_typed_different_types_returns_false() {
    assert!(!equal(Some(&"42"), Some(&SAMPLE_VALUE)));
}

#[test]
fn try_convert_typed_different_values_returns_false() {
    assert!(!equal(Some(&"test"), Some(&"other")));
}

#[test]
fn try_convert_typed_equal_integers_returns_true() {
    assert!(equal(Some(&42), Some(&SAMPLE_VALUE)));
}

#[test]
fn try_convert_typed_equal_strings_returns_true() {
    // A `String` and a `&str` compare as strings.
    let value = String::from("hello");
    assert!(equal(Some(&value), Some(&"hello")));
}

#[test]
fn try_convert_typed_equal_values_returns_true() {
    assert!(equal(Some(&"test"), Some(&"test")));
}

#[test]
fn try_convert_typed_no_conversion_hint_use_null_comparison() {
    assert!(!equal(Some(&"test"), None));
}

#[test]
fn try_convert_typed_one_null_returns_false() {
    assert!(!equal(Some(&"test"), None));
    assert!(!equal(None, Some(&"test")));
}

#[test]
fn try_convert_typed_reference_equality_returns_true() {
    struct Object;
    let object = Object;
    assert!(equal(Some(&object), Some(&object)));
}

// Not ported: TryConvertTyped_ValueEquality_ReturnsTrue - reflection (the compiler-generated `Equals` of anonymous types; the equality of type-erased values is known for the standard types only)
