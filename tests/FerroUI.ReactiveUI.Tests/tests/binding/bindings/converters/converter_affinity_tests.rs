//! `ConverterAffinityTests` (`ReactiveUI.Binding.Tests/Bindings/Converters/ConverterAffinityTests.cs`).

use ferroui_reactiveui::*;

const LAST_RESORT_AFFINITY: i32 = 1;
const STRING_CONVERTER_AFFINITY: i32 = 2;

fn assert_affinity(converter: &dyn IBindingTypeConverter, expected_affinity: i32) {
    assert_eq!(converter.get_affinity_for_objects(), expected_affinity);
}

#[test]
fn equality_converter_should_have_affinity1() {
    assert_affinity(&EqualityTypeConverter, LAST_RESORT_AFFINITY);
}

// ===================================================================
// String identity converter
// ===================================================================

#[test]
fn string_converter_should_have_affinity2() {
    assert_affinity(&StringConverter, STRING_CONVERTER_AFFINITY);
}

// ===================================================================
// Numeric to String converters
// ===================================================================

#[test]
fn byte_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&ByteToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_byte_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableByteToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn short_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&ShortToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_short_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableShortToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn integer_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&IntegerToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_integer_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableIntegerToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn long_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&LongToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_long_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableLongToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn single_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&SingleToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_single_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableSingleToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn double_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&DoubleToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_double_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableDoubleToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

// Not ported: DecimalToStringTypeConverter_ShouldHaveAffinity2 - needs Decimal in ferroui-system (phase 1b)
// Not ported: NullableDecimalToStringTypeConverter_ShouldHaveAffinity2 - needs Decimal in ferroui-system (phase 1b)

// ===================================================================
// String to Numeric converters
// ===================================================================

#[test]
fn string_to_byte_type_converter_should_have_affinity2() {
    assert_affinity(&StringToByteTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_byte_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableByteTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_short_type_converter_should_have_affinity2() {
    assert_affinity(&StringToShortTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_short_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableShortTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_integer_type_converter_should_have_affinity2() {
    assert_affinity(&StringToIntegerTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_integer_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableIntegerTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_long_type_converter_should_have_affinity2() {
    assert_affinity(&StringToLongTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_long_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableLongTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_single_type_converter_should_have_affinity2() {
    assert_affinity(&StringToSingleTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_single_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableSingleTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_double_type_converter_should_have_affinity2() {
    assert_affinity(&StringToDoubleTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_double_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableDoubleTypeConverter, STRING_CONVERTER_AFFINITY);
}

// Not ported: StringToDecimalTypeConverter_ShouldHaveAffinity2 - needs Decimal in ferroui-system (phase 1b)
// Not ported: StringToNullableDecimalTypeConverter_ShouldHaveAffinity2 - needs Decimal in ferroui-system (phase 1b)

// ===================================================================
// Boolean and String converters
// ===================================================================

#[test]
fn boolean_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&BooleanToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_boolean_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableBooleanToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_boolean_type_converter_should_have_affinity2() {
    assert_affinity(&StringToBooleanTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_boolean_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableBooleanTypeConverter, STRING_CONVERTER_AFFINITY);
}

// ===================================================================
// Guid and String converters
// ===================================================================

#[test]
fn guid_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&GuidToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_guid_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableGuidToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_guid_type_converter_should_have_affinity2() {
    assert_affinity(&StringToGuidTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_guid_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableGuidTypeConverter, STRING_CONVERTER_AFFINITY);
}

// ===================================================================
// DateTime and DateTimeOffset converters
// ===================================================================

// Not ported: DateTimeToStringTypeConverter_ShouldHaveAffinity2 - needs DateTime in ferroui-system (phase 1b)
// Not ported: NullableDateTimeToStringTypeConverter_ShouldHaveAffinity2 - needs DateTime in ferroui-system (phase 1b)
// Not ported: StringToDateTimeTypeConverter_ShouldHaveAffinity2 - needs DateTime in ferroui-system (phase 1b)
// Not ported: StringToNullableDateTimeTypeConverter_ShouldHaveAffinity2 - needs DateTime in ferroui-system (phase 1b)
// Not ported: DateTimeOffsetToStringTypeConverter_ShouldHaveAffinity2 - needs DateTimeOffset in ferroui-system (phase 1b)
// Not ported: NullableDateTimeOffsetToStringTypeConverter_ShouldHaveAffinity2 - needs DateTimeOffset in ferroui-system (phase 1b)
// Not ported: StringToDateTimeOffsetTypeConverter_ShouldHaveAffinity2 - needs DateTimeOffset in ferroui-system (phase 1b)
// Not ported: StringToNullableDateTimeOffsetTypeConverter_ShouldHaveAffinity2 - needs DateTimeOffset in ferroui-system (phase 1b)

// ===================================================================
// TimeSpan and String converters
// ===================================================================

#[test]
fn time_span_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&TimeSpanToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn nullable_time_span_to_string_type_converter_should_have_affinity2() {
    assert_affinity(&NullableTimeSpanToStringTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_time_span_type_converter_should_have_affinity2() {
    assert_affinity(&StringToTimeSpanTypeConverter, STRING_CONVERTER_AFFINITY);
}

#[test]
fn string_to_nullable_time_span_type_converter_should_have_affinity2() {
    assert_affinity(&StringToNullableTimeSpanTypeConverter, STRING_CONVERTER_AFFINITY);
}

// ===================================================================
// DateOnly, TimeOnly and Uri converters
// ===================================================================

// Not ported: DateOnlyToStringTypeConverter_ShouldHaveAffinity2 - needs DateOnly in ferroui-system (phase 1b)
// Not ported: NullableDateOnlyToStringTypeConverter_ShouldHaveAffinity2 - needs DateOnly in ferroui-system (phase 1b)
// Not ported: StringToDateOnlyTypeConverter_ShouldHaveAffinity2 - needs DateOnly in ferroui-system (phase 1b)
// Not ported: StringToNullableDateOnlyTypeConverter_ShouldHaveAffinity2 - needs DateOnly in ferroui-system (phase 1b)
// Not ported: TimeOnlyToStringTypeConverter_ShouldHaveAffinity2 - needs TimeOnly in ferroui-system (phase 1b)
// Not ported: NullableTimeOnlyToStringTypeConverter_ShouldHaveAffinity2 - needs TimeOnly in ferroui-system (phase 1b)
// Not ported: StringToTimeOnlyTypeConverter_ShouldHaveAffinity2 - needs TimeOnly in ferroui-system (phase 1b)
// Not ported: StringToNullableTimeOnlyTypeConverter_ShouldHaveAffinity2 - needs TimeOnly in ferroui-system (phase 1b)
// Not ported: UriToStringTypeConverter_ShouldHaveAffinity2 - needs Uri in ferroui-system (phase 1b)
// Not ported: StringToUriTypeConverter_ShouldHaveAffinity2 - needs Uri in ferroui-system (phase 1b)
