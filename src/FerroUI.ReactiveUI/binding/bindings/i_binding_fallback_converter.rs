//! `IBindingFallbackConverter` (`ReactiveUI.Binding/Bindings/IBindingFallbackConverter.cs`).

use crate::ConverterType;
use std::any::Any;
use std::rc::Rc;

/// A converter that the bindings ask when no typed converter handles a pair of types
/// (`IBindingFallbackConverter`). It decides at run time, from the two types, whether and how well
/// it converts; [`BindingFallbackConverterRegistry`](crate::BindingFallbackConverterRegistry) picks
/// the one with the highest affinity.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's interface extends `IEnableLogger`; the logger of
// Splat is reached through `ferroui_splat::LogHost` instead. `TryConvert(.., out object? result)`
// returns the converted value, or `None` when the conversion fails, so a fallback converter cannot
// report a success without a value.
pub trait IBindingFallbackConverter {
    /// How well this converter converts from `from_type` to `to_type`: a positive score when it
    /// can, where a higher score wins; zero or less when it cannot (`GetAffinityForObjects`).
    fn get_affinity_for_objects(&self, from_type: ConverterType, to_type: ConverterType) -> i32;

    /// Converts `from`, a value of `from_type`, to a value of `to_type`, with an optional
    /// `conversion_hint`; `None` when the conversion fails (`TryConvert`).
    fn try_convert(
        &self,
        from_type: ConverterType,
        from: &dyn Any,
        to_type: ConverterType,
        conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>>;
}
