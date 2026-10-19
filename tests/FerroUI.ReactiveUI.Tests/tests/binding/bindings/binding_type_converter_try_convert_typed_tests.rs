//! `BindingTypeConverterTryConvertTypedTests` (`ReactiveUI.Binding.Tests/Bindings/BindingTypeConverterTryConvertTypedTests.cs`).
//!
//! An input of a nullable converter is an `Option` here (upstream boxes the value itself); the
//! converters of nullables to their values take the plain value as well, as upstream's do. A `null`
//! string is an empty string.

use super::type_converters::converted;
use ferroui_reactiveui::*;
use std::any::Any;
use std::fmt::Debug;

const WRONG_TYPE_INPUT: &str = "wrong";
const BYTE_VALUE: u8 = 42;
const SMALL_BYTE_VALUE: u8 = 7;
const INT_VALUE: i32 = 123;
const SMALL_INT_VALUE: i32 = 42;
const PARSED_INT_VALUE: i32 = 456;
// Upstream's value, which happens to be close to a constant of `std::f64::consts`.
#[allow(clippy::approx_constant)]
const DOUBLE_VALUE: f64 = 3.14;
// Upstream's value, which happens to be close to a constant of `std::f64::consts`.
#[allow(clippy::approx_constant)]
const EULER_DOUBLE_VALUE: f64 = 2.718;
const LONG_VALUE: i64 = 9_876_543_210;
const SHORT_VALUE: i16 = 321;
const SINGLE_VALUE: f32 = 1.5;

fn assert_converter_success<T: Clone + Debug + PartialEq + 'static>(
    converter: &dyn IBindingTypeConverter,
    input: Option<&dyn Any>,
    expected_output: T,
) {
    let result = converter.try_convert_typed(input, None);
    assert!(result.is_some());
    assert_eq!(converted::<T>(result), Some(expected_output));
}

fn assert_converter_failure(converter: &dyn IBindingTypeConverter, input: Option<&dyn Any>) {
    assert!(converter.try_convert_typed(input, None).is_none());
}

// ===================================================================
// Non-nullable to String converters
// ===================================================================

#[test]
fn byte_to_string_success() {
    assert_converter_success(&ByteToStringTypeConverter, Some(&BYTE_VALUE), "42".to_string());
}

#[test]
fn byte_to_string_failure_wrong_type() {
    assert_converter_failure(&ByteToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn byte_to_string_failure_null() {
    assert_converter_failure(&ByteToStringTypeConverter, None);
}

#[test]
fn integer_to_string_success() {
    assert_converter_success(&IntegerToStringTypeConverter, Some(&INT_VALUE), "123".to_string());
}

#[test]
fn integer_to_string_failure_wrong_type() {
    assert_converter_failure(&IntegerToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn integer_to_string_failure_null() {
    assert_converter_failure(&IntegerToStringTypeConverter, None);
}

#[test]
fn string_to_integer_success() {
    assert_converter_success(&StringToIntegerTypeConverter, Some(&"456".to_string()), PARSED_INT_VALUE);
}

#[test]
fn boolean_to_string_success() {
    assert_converter_success(&BooleanToStringTypeConverter, Some(&true), "True".to_string());
}

#[test]
fn double_to_string_success() {
    assert_converter_success(&DoubleToStringTypeConverter, Some(&DOUBLE_VALUE), "3.14".to_string());
}

#[test]
fn double_to_string_failure_wrong_type() {
    assert_converter_failure(&DoubleToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn double_to_string_failure_null() {
    assert_converter_failure(&DoubleToStringTypeConverter, None);
}

#[test]
fn long_to_string_success() {
    assert_converter_success(&LongToStringTypeConverter, Some(&LONG_VALUE), "9876543210".to_string());
}

#[test]
fn long_to_string_failure_wrong_type() {
    assert_converter_failure(&LongToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn long_to_string_failure_null() {
    assert_converter_failure(&LongToStringTypeConverter, None);
}

#[test]
fn short_to_string_success() {
    assert_converter_success(&ShortToStringTypeConverter, Some(&SHORT_VALUE), "321".to_string());
}

#[test]
fn short_to_string_failure_wrong_type() {
    assert_converter_failure(&ShortToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn short_to_string_failure_null() {
    assert_converter_failure(&ShortToStringTypeConverter, None);
}

#[test]
fn single_to_string_success() {
    assert_converter_success(&SingleToStringTypeConverter, Some(&SINGLE_VALUE), "1.5".to_string());
}

#[test]
fn single_to_string_failure_wrong_type() {
    assert_converter_failure(&SingleToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn single_to_string_failure_null() {
    assert_converter_failure(&SingleToStringTypeConverter, None);
}

// ===================================================================
// Nullable to String converters
// ===================================================================

#[test]
fn nullable_byte_to_string_success() {
    assert_converter_success(&NullableByteToStringTypeConverter, Some(&Some(BYTE_VALUE)), "42".to_string());
}

#[test]
fn nullable_byte_to_string_failure_wrong_type() {
    assert_converter_failure(&NullableByteToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_byte_to_string_success_null() {
    assert_converter_success(&NullableByteToStringTypeConverter, None, String::new());
}

// Not ported: NullableDecimalToString_Success - needs Decimal in ferroui-system (phase 1b)
// Not ported: NullableDecimalToString_Failure_WrongType - needs Decimal in ferroui-system (phase 1b)
// Not ported: NullableDecimalToString_Success_Null - needs Decimal in ferroui-system (phase 1b)

#[test]
fn nullable_double_to_string_success() {
    assert_converter_success(&NullableDoubleToStringTypeConverter, Some(&Some(EULER_DOUBLE_VALUE)), "2.718".to_string());
}

#[test]
fn nullable_double_to_string_failure_wrong_type() {
    assert_converter_failure(&NullableDoubleToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_double_to_string_success_null() {
    assert_converter_success(&NullableDoubleToStringTypeConverter, None, String::new());
}

#[test]
fn nullable_integer_to_string_success() {
    assert_converter_success(&NullableIntegerToStringTypeConverter, Some(&Some(INT_VALUE)), "123".to_string());
}

#[test]
fn nullable_integer_to_string_failure_wrong_type() {
    assert_converter_failure(&NullableIntegerToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_integer_to_string_success_null() {
    assert_converter_success(&NullableIntegerToStringTypeConverter, None, String::new());
}

#[test]
fn nullable_long_to_string_success() {
    assert_converter_success(&NullableLongToStringTypeConverter, Some(&Some(LONG_VALUE)), "9876543210".to_string());
}

#[test]
fn nullable_long_to_string_failure_wrong_type() {
    assert_converter_failure(&NullableLongToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_long_to_string_success_null() {
    assert_converter_success(&NullableLongToStringTypeConverter, None, String::new());
}

#[test]
fn nullable_short_to_string_success() {
    assert_converter_success(&NullableShortToStringTypeConverter, Some(&Some(SHORT_VALUE)), "321".to_string());
}

#[test]
fn nullable_short_to_string_failure_wrong_type() {
    assert_converter_failure(&NullableShortToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_short_to_string_success_null() {
    assert_converter_success(&NullableShortToStringTypeConverter, None, String::new());
}

#[test]
fn nullable_single_to_string_success() {
    assert_converter_success(&NullableSingleToStringTypeConverter, Some(&Some(SINGLE_VALUE)), "1.5".to_string());
}

#[test]
fn nullable_single_to_string_failure_wrong_type() {
    assert_converter_failure(&NullableSingleToStringTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_single_to_string_success_null() {
    assert_converter_success(&NullableSingleToStringTypeConverter, None, String::new());
}

// ===================================================================
// Value-to-Nullable converters
// ===================================================================

#[test]
fn double_to_nullable_double_success() {
    assert_converter_success(&DoubleToNullableDoubleTypeConverter, Some(&DOUBLE_VALUE), Some(DOUBLE_VALUE));
}

#[test]
fn double_to_nullable_double_failure_wrong_type() {
    assert_converter_failure(&DoubleToNullableDoubleTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn double_to_nullable_double_failure_null() {
    assert_converter_failure(&DoubleToNullableDoubleTypeConverter, None);
}

#[test]
fn integer_to_nullable_integer_success() {
    assert_converter_success(&IntegerToNullableIntegerTypeConverter, Some(&SMALL_INT_VALUE), Some(SMALL_INT_VALUE));
}

#[test]
fn integer_to_nullable_integer_failure_wrong_type() {
    assert_converter_failure(&IntegerToNullableIntegerTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn integer_to_nullable_integer_failure_null() {
    assert_converter_failure(&IntegerToNullableIntegerTypeConverter, None);
}

#[test]
fn byte_to_nullable_byte_success() {
    assert_converter_success(&ByteToNullableByteTypeConverter, Some(&SMALL_BYTE_VALUE), Some(SMALL_BYTE_VALUE));
}

#[test]
fn byte_to_nullable_byte_failure_wrong_type() {
    assert_converter_failure(&ByteToNullableByteTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn byte_to_nullable_byte_failure_null() {
    assert_converter_failure(&ByteToNullableByteTypeConverter, None);
}

#[test]
fn long_to_nullable_long_success() {
    assert_converter_success(&LongToNullableLongTypeConverter, Some(&LONG_VALUE), Some(LONG_VALUE));
}

#[test]
fn long_to_nullable_long_failure_wrong_type() {
    assert_converter_failure(&LongToNullableLongTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn long_to_nullable_long_failure_null() {
    assert_converter_failure(&LongToNullableLongTypeConverter, None);
}

#[test]
fn short_to_nullable_short_success() {
    assert_converter_success(&ShortToNullableShortTypeConverter, Some(&SHORT_VALUE), Some(SHORT_VALUE));
}

#[test]
fn short_to_nullable_short_failure_wrong_type() {
    assert_converter_failure(&ShortToNullableShortTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn short_to_nullable_short_failure_null() {
    assert_converter_failure(&ShortToNullableShortTypeConverter, None);
}

#[test]
fn single_to_nullable_single_success() {
    assert_converter_success(&SingleToNullableSingleTypeConverter, Some(&SINGLE_VALUE), Some(SINGLE_VALUE));
}

#[test]
fn single_to_nullable_single_failure_wrong_type() {
    assert_converter_failure(&SingleToNullableSingleTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn single_to_nullable_single_failure_null() {
    assert_converter_failure(&SingleToNullableSingleTypeConverter, None);
}

// Not ported: DecimalToNullableDecimal_Success - needs Decimal in ferroui-system (phase 1b)
// Not ported: DecimalToNullableDecimal_Failure_WrongType - needs Decimal in ferroui-system (phase 1b)
// Not ported: DecimalToNullableDecimal_Failure_Null - needs Decimal in ferroui-system (phase 1b)

// ===================================================================
// Nullable-to-Value converters
// ===================================================================

#[test]
fn nullable_double_to_double_success() {
    assert_converter_success(&NullableDoubleToDoubleTypeConverter, Some(&DOUBLE_VALUE), DOUBLE_VALUE);
}

#[test]
fn nullable_double_to_double_failure_wrong_type() {
    assert_converter_failure(&NullableDoubleToDoubleTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_double_to_double_failure_null() {
    assert_converter_failure(&NullableDoubleToDoubleTypeConverter, None);
}

#[test]
fn nullable_integer_to_integer_success() {
    assert_converter_success(&NullableIntegerToIntegerTypeConverter, Some(&SMALL_INT_VALUE), SMALL_INT_VALUE);
}

#[test]
fn nullable_integer_to_integer_failure_wrong_type() {
    assert_converter_failure(&NullableIntegerToIntegerTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_integer_to_integer_failure_null() {
    assert_converter_failure(&NullableIntegerToIntegerTypeConverter, None);
}

#[test]
fn nullable_byte_to_byte_success() {
    assert_converter_success(&NullableByteToByteTypeConverter, Some(&SMALL_BYTE_VALUE), SMALL_BYTE_VALUE);
}

#[test]
fn nullable_byte_to_byte_failure_wrong_type() {
    assert_converter_failure(&NullableByteToByteTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_byte_to_byte_failure_null() {
    assert_converter_failure(&NullableByteToByteTypeConverter, None);
}

#[test]
fn nullable_long_to_long_success() {
    assert_converter_success(&NullableLongToLongTypeConverter, Some(&LONG_VALUE), LONG_VALUE);
}

#[test]
fn nullable_long_to_long_failure_wrong_type() {
    assert_converter_failure(&NullableLongToLongTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_long_to_long_failure_null() {
    assert_converter_failure(&NullableLongToLongTypeConverter, None);
}

#[test]
fn nullable_short_to_short_success() {
    assert_converter_success(&NullableShortToShortTypeConverter, Some(&SHORT_VALUE), SHORT_VALUE);
}

#[test]
fn nullable_short_to_short_failure_wrong_type() {
    assert_converter_failure(&NullableShortToShortTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_short_to_short_failure_null() {
    assert_converter_failure(&NullableShortToShortTypeConverter, None);
}

#[test]
fn nullable_single_to_single_success() {
    assert_converter_success(&NullableSingleToSingleTypeConverter, Some(&SINGLE_VALUE), SINGLE_VALUE);
}

#[test]
fn nullable_single_to_single_failure_wrong_type() {
    assert_converter_failure(&NullableSingleToSingleTypeConverter, Some(&WRONG_TYPE_INPUT));
}

#[test]
fn nullable_single_to_single_failure_null() {
    assert_converter_failure(&NullableSingleToSingleTypeConverter, None);
}

// Not ported: NullableDecimalToDecimal_Success - needs Decimal in ferroui-system (phase 1b)
// Not ported: NullableDecimalToDecimal_Failure_WrongType - needs Decimal in ferroui-system (phase 1b)
// Not ported: NullableDecimalToDecimal_Failure_Null - needs Decimal in ferroui-system (phase 1b)
