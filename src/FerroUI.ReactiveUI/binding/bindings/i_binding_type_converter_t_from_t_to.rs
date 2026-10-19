//! `IBindingTypeConverter<TFrom, TTo>`
//! (`ReactiveUI.Binding/Bindings/IBindingTypeConverter{TFrom,TTo}.cs`).

use crate::IBindingTypeConverter;
use std::any::Any;

/// A converter from `F` to `T` with a typed conversion (`IBindingTypeConverter<TFrom, TTo>`).
///
/// Every converter derived from [`BindingTypeConverter`](crate::BindingTypeConverter) implements it
/// with its `From` and `To` types; a converter that implements [`IBindingTypeConverter`] directly
/// implements it as well.
pub trait IBindingTypeConverterOf<F: 'static, T: 'static>: IBindingTypeConverter {
    /// Converts `from` (`None` for upstream's `null`, which only a converter of a nullable source
    /// accepts), with an optional `conversion_hint`, and returns the converted value, or `None`
    /// when the conversion fails (`TryConvert(TFrom? from, object? conversionHint, out TTo? result)`).
    fn try_convert(&self, from: Option<&F>, conversion_hint: Option<&dyn Any>) -> Option<T>;
}
