//! `BindingTypeConverterDispatch` (`ReactiveUI.Binding/Bindings/BindingTypeConverterDispatch.cs`).

use crate::{ConverterType, IBindingFallbackConverter, IBindingTypeConverter, ResolvedConverter};
use std::any::Any;
use std::rc::Rc;

/// Runs a converter found at run time, typed or fallback, with the checks the bindings rely on
/// (`BindingTypeConverterDispatch`).
pub struct BindingTypeConverterDispatch;

impl BindingTypeConverterDispatch {
    /// Converts `from` with a typed converter, when the converter's target type is `to_type` and
    /// `from` is a value of its source type exactly; `None` otherwise or when the conversion fails
    /// (`TryConvert(IBindingTypeConverter, ..)`).
    ///
    /// A `None` input (upstream's `null`) goes to the converter, which decides.
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream fails a `null` input first when the source
    // type is a non-nullable value type, and accepts a boxed `T` for a converter of `Nullable<T>`.
    // Both read the type by reflection (`Type.IsValueType`, `Nullable.GetUnderlyingType`); here a
    // value keeps its `Option`, so the source type is matched exactly, and the converters of value
    // types fail on `None` themselves.
    pub fn try_convert(
        converter: &dyn IBindingTypeConverter,
        from: Option<&dyn Any>,
        to_type: ConverterType,
        conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        if converter.to_type() != to_type {
            return None;
        }
        if let Some(value) = from {
            // An exact match keeps the dispatch predictable.
            if !converter.from_type().is_type_of(value) {
                return None;
            }
        }
        converter.try_convert_typed(from, conversion_hint)
    }

    /// Converts `from` with a fallback converter (`TryConvertFallback`).
    pub fn try_convert_fallback(
        converter: &dyn IBindingFallbackConverter,
        from_type: ConverterType,
        from: &dyn Any,
        to_type: ConverterType,
        conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        converter.try_convert(from_type, from, to_type, conversion_hint)
    }

    /// Converts `from` with whatever [`ConverterService::resolve_converter`] found: a typed
    /// converter goes through [`try_convert`](Self::try_convert), a fallback converter through
    /// [`try_convert_fallback`](Self::try_convert_fallback) and fails a `None` input; no converter
    /// fails (`TryConvertAny`).
    ///
    /// [`ConverterService::resolve_converter`]: crate::ConverterService::resolve_converter
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream takes an `object?` and fails an object that
    // is neither kind of converter; the enum `ResolvedConverter` has no such case.
    pub fn try_convert_any(
        converter: Option<&ResolvedConverter>,
        from_type: ConverterType,
        from: Option<&dyn Any>,
        to_type: ConverterType,
        conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        match converter? {
            ResolvedConverter::Typed(typed) => Self::try_convert(&**typed, from, to_type, conversion_hint),
            ResolvedConverter::Fallback(fallback) => {
                Self::try_convert_fallback(&**fallback, from_type, from?, to_type, conversion_hint)
            }
        }
    }
}
