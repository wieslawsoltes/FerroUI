//! `DefaultConverterRegistration`
//! (`ReactiveUI.Binding/Bindings/Converters/DefaultConverterRegistration.cs`).

use crate::*;
use std::rc::Rc;

/// Registers the built-in converters in a [`ConverterService`] (`DefaultConverterRegistration`).
pub struct DefaultConverterRegistration;

impl DefaultConverterRegistration {
    /// Registers the built-in typed converters in the typed registry of `service`
    /// (`RegisterDefaults`): the identity converter of strings, the equality converter, and the
    /// converters between strings and the booleans, identifiers, numbers and time spans, in both
    /// directions, and between each number type and its `Option`.
    // Deviation (DEVIATIONS.md, ReactiveUI): the converters of `Uri`, `decimal`, `DateTime`,
    // `DateTimeOffset`, `DateOnly` and `TimeOnly` wait for those types in ferroui-system
    // (phase 1b); `Guid` is `ferroui_splat::Guid` and `TimeSpan` is `std::time::Duration`.
    pub fn register_defaults(service: &ConverterService) {
        let registry = service.typed_converters();
        register_core_converters(registry);
        register_numeric_converters(registry);
        register_temporal_converters(registry);
    }
}

/// Registers the built-in converters in `service`: the same as
/// [`DefaultConverterRegistration::register_defaults`].
pub fn register_default_converters(service: &ConverterService) {
    DefaultConverterRegistration::register_defaults(service);
}

fn register_core_converters(registry: &BindingTypeConverterRegistry) {
    // String identity converter
    registry.register(Rc::new(StringConverter));

    // Equality converter (last resort, affinity 1)
    registry.register(Rc::new(EqualityTypeConverter));

    // Boolean converters
    registry.register(Rc::new(BooleanToStringTypeConverter));
    registry.register(Rc::new(StringToBooleanTypeConverter));
    registry.register(Rc::new(NullableBooleanToStringTypeConverter));
    registry.register(Rc::new(StringToNullableBooleanTypeConverter));

    // Guid converters
    registry.register(Rc::new(GuidToStringTypeConverter));
    registry.register(Rc::new(StringToGuidTypeConverter));
    registry.register(Rc::new(NullableGuidToStringTypeConverter));
    registry.register(Rc::new(StringToNullableGuidTypeConverter));

    // Uri converters: UriToStringTypeConverter and StringToUriTypeConverter need Uri in
    // ferroui-system (phase 1b).
}

fn register_numeric_converters(registry: &BindingTypeConverterRegistry) {
    // Integer converters
    registry.register(Rc::new(IntegerToStringTypeConverter));
    registry.register(Rc::new(StringToIntegerTypeConverter));
    registry.register(Rc::new(NullableIntegerToIntegerTypeConverter));
    registry.register(Rc::new(IntegerToNullableIntegerTypeConverter));
    registry.register(Rc::new(NullableIntegerToStringTypeConverter));
    registry.register(Rc::new(StringToNullableIntegerTypeConverter));

    // Double converters
    registry.register(Rc::new(DoubleToStringTypeConverter));
    registry.register(Rc::new(StringToDoubleTypeConverter));
    registry.register(Rc::new(NullableDoubleToDoubleTypeConverter));
    registry.register(Rc::new(DoubleToNullableDoubleTypeConverter));
    registry.register(Rc::new(NullableDoubleToStringTypeConverter));
    registry.register(Rc::new(StringToNullableDoubleTypeConverter));

    // Single (float) converters
    registry.register(Rc::new(SingleToStringTypeConverter));
    registry.register(Rc::new(StringToSingleTypeConverter));
    registry.register(Rc::new(NullableSingleToSingleTypeConverter));
    registry.register(Rc::new(SingleToNullableSingleTypeConverter));
    registry.register(Rc::new(NullableSingleToStringTypeConverter));
    registry.register(Rc::new(StringToNullableSingleTypeConverter));

    // Decimal converters: DecimalToStringTypeConverter, StringToDecimalTypeConverter,
    // NullableDecimalToDecimalTypeConverter, DecimalToNullableDecimalTypeConverter,
    // NullableDecimalToStringTypeConverter and StringToNullableDecimalTypeConverter need Decimal in
    // ferroui-system (phase 1b).

    // Long converters
    registry.register(Rc::new(LongToStringTypeConverter));
    registry.register(Rc::new(StringToLongTypeConverter));
    registry.register(Rc::new(NullableLongToLongTypeConverter));
    registry.register(Rc::new(LongToNullableLongTypeConverter));
    registry.register(Rc::new(NullableLongToStringTypeConverter));
    registry.register(Rc::new(StringToNullableLongTypeConverter));

    // Short converters
    registry.register(Rc::new(ShortToStringTypeConverter));
    registry.register(Rc::new(StringToShortTypeConverter));
    registry.register(Rc::new(NullableShortToShortTypeConverter));
    registry.register(Rc::new(ShortToNullableShortTypeConverter));
    registry.register(Rc::new(NullableShortToStringTypeConverter));
    registry.register(Rc::new(StringToNullableShortTypeConverter));

    // Byte converters
    registry.register(Rc::new(ByteToStringTypeConverter));
    registry.register(Rc::new(StringToByteTypeConverter));
    registry.register(Rc::new(NullableByteToByteTypeConverter));
    registry.register(Rc::new(ByteToNullableByteTypeConverter));
    registry.register(Rc::new(NullableByteToStringTypeConverter));
    registry.register(Rc::new(StringToNullableByteTypeConverter));
}

fn register_temporal_converters(registry: &BindingTypeConverterRegistry) {
    // DateTime and DateTimeOffset converters (DateTimeToStringTypeConverter, ...) need DateTime and
    // DateTimeOffset in ferroui-system (phase 1b).

    // TimeSpan converters
    registry.register(Rc::new(TimeSpanToStringTypeConverter));
    registry.register(Rc::new(StringToTimeSpanTypeConverter));
    registry.register(Rc::new(NullableTimeSpanToStringTypeConverter));
    registry.register(Rc::new(StringToNullableTimeSpanTypeConverter));

    // DateOnly and TimeOnly converters (DateOnlyToStringTypeConverter, ...) need DateOnly and
    // TimeOnly in ferroui-system (phase 1b).
}
