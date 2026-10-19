//! The tests of `ReactiveUI.Binding.Tests/Bindings/TypeConverters`: one module per upstream test
//! file, with the helpers the tests of the converters share.
//!
//! A type-erased conversion returns the converted value or `None`; [`converted`] reads the value
//! as the type a test expects. Upstream's `null` input is `None`, and its `null` output of the
//! converters of nullables to strings is an empty string here.

use std::any::Any;
use std::rc::Rc;

/// The value of a type-erased conversion as a `T`; `None` when the conversion failed or produced
/// another type.
pub(crate) fn converted<T: Clone + 'static>(result: Option<Rc<dyn Any>>) -> Option<T> {
    result.and_then(|value| value.downcast_ref::<T>().cloned())
}

/// Whether two `Rc`s point to the same object, whatever their types (`ReferenceEquals`).
pub(crate) fn same_object<T: ?Sized, U: ?Sized>(left: &Rc<T>, right: &Rc<U>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(left), Rc::as_ptr(right))
}

/// Asserts that `actual` is within `tolerance` of `expected` (`IsEqualTo(..).Within(..)`).
pub(crate) fn assert_within(actual: f32, expected: f32, tolerance: f32) {
    assert!((actual - expected).abs() <= tolerance, "{actual} is not within {tolerance} of {expected}");
}

mod binding_type_converter_base_tests;
mod boolean_to_string_type_converter_tests;
mod byte_to_nullable_byte_type_converter_tests;
mod byte_to_string_type_converter_tests;
mod date_only_to_string_type_converter_tests;
mod date_time_offset_to_string_type_converter_tests;
mod date_time_to_string_type_converter_tests;
mod decimal_to_nullable_decimal_type_converter_tests;
mod decimal_to_string_type_converter_tests;
mod double_to_nullable_double_type_converter_tests;
mod double_to_string_type_converter_tests;
mod equality_type_converter_tests;
mod guid_to_string_type_converter_tests;
mod integer_to_nullable_integer_type_converter_tests;
mod integer_to_string_type_converter_tests;
mod long_to_nullable_long_type_converter_tests;
mod long_to_string_type_converter_tests;
mod nullable_boolean_to_string_type_converter_tests;
mod nullable_byte_to_byte_type_converter_tests;
mod nullable_byte_to_string_type_converter_tests;
mod nullable_date_only_to_string_type_converter_tests;
mod nullable_date_time_offset_to_string_type_converter_tests;
mod nullable_date_time_to_string_type_converter_tests;
mod nullable_decimal_to_decimal_type_converter_tests;
mod nullable_decimal_to_string_type_converter_tests;
mod nullable_double_to_double_type_converter_tests;
mod nullable_double_to_string_type_converter_tests;
mod nullable_guid_to_string_type_converter_tests;
mod nullable_integer_to_integer_type_converter_tests;
mod nullable_integer_to_string_type_converter_tests;
mod nullable_long_to_long_type_converter_tests;
mod nullable_long_to_string_type_converter_tests;
mod nullable_short_to_short_type_converter_tests;
mod nullable_short_to_string_type_converter_tests;
mod nullable_single_to_single_type_converter_tests;
mod nullable_single_to_string_type_converter_tests;
mod nullable_time_only_to_string_type_converter_tests;
mod nullable_time_span_to_string_type_converter_tests;
mod numeric_to_string_culture_tests;
mod short_to_nullable_short_type_converter_tests;
mod short_to_string_type_converter_tests;
mod single_to_nullable_single_type_converter_tests;
mod single_to_string_type_converter_tests;
mod string_converter_tests;
mod string_to_boolean_type_converter_tests;
mod string_to_byte_type_converter_tests;
mod string_to_date_only_type_converter_tests;
mod string_to_date_time_offset_type_converter_tests;
mod string_to_date_time_type_converter_tests;
mod string_to_decimal_type_converter_tests;
mod string_to_double_type_converter_tests;
mod string_to_guid_type_converter_tests;
mod string_to_integer_type_converter_tests;
mod string_to_long_type_converter_tests;
mod string_to_nullable_boolean_type_converter_tests;
mod string_to_nullable_byte_type_converter_tests;
mod string_to_nullable_date_only_type_converter_tests;
mod string_to_nullable_date_time_offset_type_converter_tests;
mod string_to_nullable_date_time_type_converter_tests;
mod string_to_nullable_decimal_type_converter_tests;
mod string_to_nullable_double_type_converter_tests;
mod string_to_nullable_guid_type_converter_tests;
mod string_to_nullable_integer_type_converter_tests;
mod string_to_nullable_long_type_converter_tests;
mod string_to_nullable_short_type_converter_tests;
mod string_to_nullable_single_type_converter_tests;
mod string_to_nullable_time_only_type_converter_tests;
mod string_to_nullable_time_span_type_converter_tests;
mod string_to_short_type_converter_tests;
mod string_to_single_type_converter_tests;
mod string_to_time_only_type_converter_tests;
mod string_to_time_span_type_converter_tests;
mod string_to_uri_type_converter_tests;
mod time_only_to_string_type_converter_tests;
mod time_span_to_string_type_converter_tests;
mod uri_to_string_type_converter_tests;
