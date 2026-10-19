//! `ReactiveUI.Binding/Bindings/Converter`: the built-in typed converters, between strings and
//! the booleans, identifiers ([`Guid`](ferroui_splat::Guid)), numbers and time spans
//! ([`Duration`](std::time::Duration)), between each number type and its `Option`, the identity
//! converter of strings and the equality converter.
//!
//! The text of numbers follows .NET's format strings and parsing with the invariant culture
//! (`number_format`); the text of booleans, identifiers and time spans follows .NET's as well
//! (`value_text`).
//!
//! Not ported: the converters of `Uri` (`UriToStringTypeConverter`, `StringToUriTypeConverter`),
//! `decimal` (`DecimalToStringTypeConverter`, `StringToDecimalTypeConverter`,
//! `NullableDecimalToDecimalTypeConverter`, `DecimalToNullableDecimalTypeConverter`,
//! `NullableDecimalToStringTypeConverter`, `StringToNullableDecimalTypeConverter`), `DateTime`,
//! `DateTimeOffset`, `DateOnly` and `TimeOnly` (`DateTimeToStringTypeConverter`,
//! `StringToDateTimeTypeConverter`, `NullableDateTimeToStringTypeConverter`,
//! `StringToNullableDateTimeTypeConverter` and the same four of each other type): they need those
//! types in ferroui-system (phase 1b).

mod conversion_hint;
mod number_format;
mod value_text;
mod boolean_to_string_type_converter;
mod byte_to_nullable_byte_type_converter;
mod byte_to_string_type_converter;
mod double_to_nullable_double_type_converter;
mod double_to_string_type_converter;
mod equality_type_converter;
mod guid_to_string_type_converter;
mod integer_to_nullable_integer_type_converter;
mod integer_to_string_type_converter;
mod long_to_nullable_long_type_converter;
mod long_to_string_type_converter;
mod nullable_boolean_to_string_type_converter;
mod nullable_byte_to_byte_type_converter;
mod nullable_byte_to_string_type_converter;
mod nullable_double_to_double_type_converter;
mod nullable_double_to_string_type_converter;
mod nullable_guid_to_string_type_converter;
mod nullable_integer_to_integer_type_converter;
mod nullable_integer_to_string_type_converter;
mod nullable_long_to_long_type_converter;
mod nullable_long_to_string_type_converter;
mod nullable_short_to_short_type_converter;
mod nullable_short_to_string_type_converter;
mod nullable_single_to_single_type_converter;
mod nullable_single_to_string_type_converter;
mod nullable_time_span_to_string_type_converter;
mod short_to_nullable_short_type_converter;
mod short_to_string_type_converter;
mod single_to_nullable_single_type_converter;
mod single_to_string_type_converter;
mod string_converter;
mod string_to_boolean_type_converter;
mod string_to_byte_type_converter;
mod string_to_double_type_converter;
mod string_to_guid_type_converter;
mod string_to_integer_type_converter;
mod string_to_long_type_converter;
mod string_to_nullable_boolean_type_converter;
mod string_to_nullable_byte_type_converter;
mod string_to_nullable_double_type_converter;
mod string_to_nullable_guid_type_converter;
mod string_to_nullable_integer_type_converter;
mod string_to_nullable_long_type_converter;
mod string_to_nullable_short_type_converter;
mod string_to_nullable_single_type_converter;
mod string_to_nullable_time_span_type_converter;
mod string_to_short_type_converter;
mod string_to_single_type_converter;
mod string_to_time_span_type_converter;
mod time_span_to_string_type_converter;

pub use boolean_to_string_type_converter::BooleanToStringTypeConverter;
pub use byte_to_nullable_byte_type_converter::ByteToNullableByteTypeConverter;
pub use byte_to_string_type_converter::ByteToStringTypeConverter;
pub use double_to_nullable_double_type_converter::DoubleToNullableDoubleTypeConverter;
pub use double_to_string_type_converter::DoubleToStringTypeConverter;
pub use equality_type_converter::EqualityTypeConverter;
pub use guid_to_string_type_converter::GuidToStringTypeConverter;
pub use integer_to_nullable_integer_type_converter::IntegerToNullableIntegerTypeConverter;
pub use integer_to_string_type_converter::IntegerToStringTypeConverter;
pub use long_to_nullable_long_type_converter::LongToNullableLongTypeConverter;
pub use long_to_string_type_converter::LongToStringTypeConverter;
pub use nullable_boolean_to_string_type_converter::NullableBooleanToStringTypeConverter;
pub use nullable_byte_to_byte_type_converter::NullableByteToByteTypeConverter;
pub use nullable_byte_to_string_type_converter::NullableByteToStringTypeConverter;
pub use nullable_double_to_double_type_converter::NullableDoubleToDoubleTypeConverter;
pub use nullable_double_to_string_type_converter::NullableDoubleToStringTypeConverter;
pub use nullable_guid_to_string_type_converter::NullableGuidToStringTypeConverter;
pub use nullable_integer_to_integer_type_converter::NullableIntegerToIntegerTypeConverter;
pub use nullable_integer_to_string_type_converter::NullableIntegerToStringTypeConverter;
pub use nullable_long_to_long_type_converter::NullableLongToLongTypeConverter;
pub use nullable_long_to_string_type_converter::NullableLongToStringTypeConverter;
pub use nullable_short_to_short_type_converter::NullableShortToShortTypeConverter;
pub use nullable_short_to_string_type_converter::NullableShortToStringTypeConverter;
pub use nullable_single_to_single_type_converter::NullableSingleToSingleTypeConverter;
pub use nullable_single_to_string_type_converter::NullableSingleToStringTypeConverter;
pub use nullable_time_span_to_string_type_converter::NullableTimeSpanToStringTypeConverter;
pub use short_to_nullable_short_type_converter::ShortToNullableShortTypeConverter;
pub use short_to_string_type_converter::ShortToStringTypeConverter;
pub use single_to_nullable_single_type_converter::SingleToNullableSingleTypeConverter;
pub use single_to_string_type_converter::SingleToStringTypeConverter;
pub use string_converter::StringConverter;
pub use string_to_boolean_type_converter::StringToBooleanTypeConverter;
pub use string_to_byte_type_converter::StringToByteTypeConverter;
pub use string_to_double_type_converter::StringToDoubleTypeConverter;
pub use string_to_guid_type_converter::StringToGuidTypeConverter;
pub use string_to_integer_type_converter::StringToIntegerTypeConverter;
pub use string_to_long_type_converter::StringToLongTypeConverter;
pub use string_to_nullable_boolean_type_converter::StringToNullableBooleanTypeConverter;
pub use string_to_nullable_byte_type_converter::StringToNullableByteTypeConverter;
pub use string_to_nullable_double_type_converter::StringToNullableDoubleTypeConverter;
pub use string_to_nullable_guid_type_converter::StringToNullableGuidTypeConverter;
pub use string_to_nullable_integer_type_converter::StringToNullableIntegerTypeConverter;
pub use string_to_nullable_long_type_converter::StringToNullableLongTypeConverter;
pub use string_to_nullable_short_type_converter::StringToNullableShortTypeConverter;
pub use string_to_nullable_single_type_converter::StringToNullableSingleTypeConverter;
pub use string_to_nullable_time_span_type_converter::StringToNullableTimeSpanTypeConverter;
pub use string_to_short_type_converter::StringToShortTypeConverter;
pub use string_to_single_type_converter::StringToSingleTypeConverter;
pub use string_to_time_span_type_converter::StringToTimeSpanTypeConverter;
pub use time_span_to_string_type_converter::TimeSpanToStringTypeConverter;
